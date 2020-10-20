use std::sync::{
  mpsc,
  mpsc::{Receiver, Sender},
  Arc, Mutex,
};
use std::{cell::RefCell, collections::HashMap, io::Cursor, rc::Rc, thread};

use ringbuf::RingBuffer;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use stainless_ffmpeg::{
  audio_decoder::AudioDecoder, check_result, filter_graph::FilterGraph,
  format_context::FormatContext, frame::Frame, packet::Packet, tools, video_decoder::VideoDecoder,
};
use stainless_ffmpeg_sys::{
  av_frame_alloc, av_frame_clone, av_seek_frame, avcodec_receive_frame, avcodec_send_packet,
  AVSEEK_FLAG_ANY, AVSEEK_FLAG_FRAME, AV_TIME_BASE,
};

use crate::{
  error::MessageError::RuntimeError,
  job::JobResult,
  message::media::{media_stream::MediaStream, srt::SrtStream},
  AudioFilter, MessageError, MessageEvent, ProcessResult, Result, VideoFilter,
};

pub enum DecodeResult {
  EndOfStream,
  Frame { stream_index: usize, frame: Frame },
  Nothing,
  WaitMore,
}

type AsyncChannelSenderReceiver = (
  Sender<Arc<Mutex<FormatContext>>>,
  Receiver<Arc<Mutex<FormatContext>>>,
);

pub struct Source {
  decoders: HashMap<usize, Decoder>,
  format_context: Arc<Mutex<FormatContext>>,
  thread: Option<thread::JoinHandle<()>>,
  duration: Option<u64>,
  start_offset: u64,
  position: u64,
}

impl Source {
  pub fn new<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    message_event: Rc<RefCell<ME>>,
    job_result: &JobResult,
    parameters: P,
    source_url: &str,
    sender: Arc<Mutex<Sender<ProcessResult>>>,
    start_index_ms: Option<i64>,
    end_index_ms: Option<i64>,
  ) -> Result<Self> {
    info!(target: &job_result.get_str_job_id(), "Opening source: {}", source_url);

    if SrtStream::is_srt_stream(source_url) {
      let (tx, rx): AsyncChannelSenderReceiver = mpsc::channel();
      let cloned_source_url = source_url.to_string();
      let source_thread = thread::spawn(move || {
        let mut srt_stream = SrtStream::open_connection(&cloned_source_url).unwrap();

        let format = "mpegts";

        let ring_buffer = RingBuffer::<u8>::new(100 * 1024 * 1024);
        let (mut producer, consumer) = ring_buffer.split();
        let media_stream = MediaStream::new(format, consumer).unwrap();

        let mut got_stream_info = false;

        loop {
          if let Some((_instant, bytes)) = srt_stream.receive() {
            trace!("{:?}", bytes);
            let size = bytes.len();
            let mut cursor = Cursor::new(bytes);

            producer.read_from(&mut cursor, Some(size)).unwrap();

            if !got_stream_info && producer.len() > 1024 * 1024 {
              match media_stream.stream_info() {
                Err(error) => error!("{}", error),
                Ok(()) => {
                  got_stream_info = true;
                  tx.send(Arc::new(Mutex::new(FormatContext::from(
                    media_stream.format_context,
                  ))))
                  .unwrap();
                }
              }
            }
          }
        }
      });

      let format_context = rx.recv().unwrap();

      let decoders = Self::get_decoders(
        message_event,
        &job_result.get_str_job_id(),
        parameters,
        format_context.clone(),
        sender,
      )?;

      Ok(Source {
        decoders,
        format_context,
        thread: Some(source_thread),
        duration: None,
        start_offset: 0,
        position: 0,
      })
    } else {
      let mut format_context = FormatContext::new(source_url).map_err(RuntimeError)?;
      format_context.open_input().map_err(RuntimeError)?;

      // FIXME: hard-coded fps
      let default_fps = 25.0;

      let start_offset = start_index_ms.unwrap_or_else(|| 0);
      let end: Option<i64> = end_index_ms.or_else(|| {
        format_context
          .get_duration()
          .map(|seconds| (seconds * 1000.0) as i64)
      });
      let duration = end.map(|end| Self::get_duration_from_index(start_offset, end, default_fps));

      if let Some(ms) = start_index_ms {
        unsafe {
          let time_stamp = (ms * AV_TIME_BASE as i64) as f32 / 1000.0;
          if av_seek_frame(
            format_context.format_context,
            -1,
            time_stamp as i64,
            AVSEEK_FLAG_ANY | AVSEEK_FLAG_FRAME,
          ) != 0
          {
            return Err(MessageError::ProcessingError(
              job_result
                .clone()
                .with_message("Could not seek at expected position into source file."),
            ));
          }
          debug!("Seek at {}/{} in source.", time_stamp, AV_TIME_BASE);
        }
      }

      let format_context = Arc::new(Mutex::new(format_context));

      let decoders = Self::get_decoders(
        message_event,
        &job_result.get_str_job_id(),
        parameters,
        format_context.clone(),
        sender,
      )?;

      Ok(Source {
        decoders,
        format_context,
        thread: None,
        duration,
        start_offset: start_offset as u64,
        position: start_offset as u64,
      })
    }
  }

  fn get_duration_from_index(start: i64, end: i64, fps: f64) -> u64 {
    ((end - start) as f64 * fps / 1000.0) as u64
  }

  pub fn get_start_offset(&self) -> u64 {
    self.start_offset
  }

  pub fn get_duration(&self) -> Option<u64> {
    self.duration
  }

  pub fn get_first_stream_index(&self) -> usize {
    self.decoders.keys().cloned().min().unwrap_or_else(|| 0)
  }

  pub fn next_frame(&mut self) -> Result<DecodeResult> {
    // Check whether the end is not reached
    if let Some(duration) = self.duration {
      if self.position >= self.start_offset + duration {
        return Ok(DecodeResult::EndOfStream);
      }
    }

    match self.format_context.lock().unwrap().next_packet() {
      Err(message) => {
        if message == "Unable to read next packet" {
          if self.thread.is_none() {
            return Ok(DecodeResult::EndOfStream);
          } else {
            return Ok(DecodeResult::WaitMore);
          }
        }

        if message == "End of data stream" {
          Ok(DecodeResult::EndOfStream)
        } else {
          Err(RuntimeError(message))
        }
      }
      Ok(packet) => {
        let stream_index = packet.get_stream_index() as usize;

        if stream_index == self.get_first_stream_index() {
          self.position += 1;
        }

        if let Some(decoder) = self.decoders.get(&stream_index) {
          match decoder.decode(&packet) {
            Ok(frame) => Ok(DecodeResult::Frame {
              stream_index,
              frame,
            }),
            Err(message) => {
              if message == "Resource temporarily unavailable" {
                return Ok(DecodeResult::Nothing);
              }
              Err(RuntimeError(message))
            }
          }
        } else {
          Ok(DecodeResult::Nothing)
        }
      }
    }
  }

  fn get_decoders<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    message_event: Rc<RefCell<ME>>,
    job_id: &str,
    parameters: P,
    format_context: Arc<Mutex<FormatContext>>,
    sender: Arc<Mutex<Sender<ProcessResult>>>,
  ) -> Result<HashMap<usize, Decoder>> {
    let selected_streams =
      message_event
        .borrow_mut()
        .init_process(parameters, format_context.clone(), sender)?;

    info!(
      target: job_id,
      "Selected stream IDs: {:?}", selected_streams
    );

    let mut decoders = HashMap::<usize, Decoder>::new();
    for selected_stream in &selected_streams {
      if let Some(audio_configuration) = &selected_stream.audio_configuration {
        // AudioDecoder can decode any codec, not only video
        let audio_decoder = AudioDecoder::new(
          format!("decoder_{}", selected_stream.index),
          &format_context.clone().lock().unwrap(),
          selected_stream.index as isize,
        )
        .map_err(RuntimeError)?;

        let audio_graph =
          Source::get_audio_filter_graph(&audio_configuration.filters, &audio_decoder)?;

        let decoder = Decoder {
          audio_decoder: Some(audio_decoder),
          video_decoder: None,
          graph: audio_graph,
        };

        decoders.insert(selected_stream.index, decoder);
      } else if let Some(image_configuration) = &selected_stream.image_configuration {
        let video_decoder = VideoDecoder::new(
          format!("decoder_{}", selected_stream.index),
          &format_context.clone().lock().unwrap(),
          selected_stream.index as isize,
        )
        .map_err(RuntimeError)?;

        let video_graph =
          Source::get_video_filter_graph(&image_configuration.filters, &video_decoder)?;

        let decoder = Decoder {
          audio_decoder: None,
          video_decoder: Some(video_decoder),
          graph: video_graph,
        };

        decoders.insert(selected_stream.index, decoder);
      }
    }
    Ok(decoders)
  }

  fn get_video_filter_graph(
    video_filters: &[VideoFilter],
    video_decoder: &VideoDecoder,
  ) -> Result<Option<FilterGraph>> {
    let mut graph = FilterGraph::new().map_err(RuntimeError)?;

    let mut filters = vec![];
    for video_filter in video_filters {
      let filter = video_filter
        .as_generic_filter(video_decoder)
        .map_err(RuntimeError)?
        .as_filter()
        .map_err(RuntimeError)?;
      filters.push(graph.add_filter(&filter).map_err(RuntimeError)?);
    }

    if !filters.is_empty() {
      graph
        .add_input_from_video_decoder("video_input", video_decoder)
        .map_err(RuntimeError)?;
      graph
        .add_video_output("video_output")
        .map_err(RuntimeError)?;

      let mut filter = filters.remove(0);
      trace!(
        "Connect video graph input to filter {}...",
        filter.get_label()
      );
      graph
        .connect_input("video_input", 0, &filter, 0)
        .map_err(RuntimeError)?;

      while !filters.is_empty() {
        let next_filter = filters.remove(0);
        trace!(
          "Connect filter {} to filter {}...",
          filter.get_label(),
          next_filter.get_label()
        );
        graph
          .connect(&filter, 0, &next_filter, 0)
          .map_err(RuntimeError)?;
        filter = next_filter;
      }

      trace!(
        "Connect filter {} to video graph output...",
        filter.get_label()
      );
      graph
        .connect_output(&filter, 0, "video_output", 0)
        .map_err(RuntimeError)?;

      graph.validate().map_err(RuntimeError)?;
      Ok(Some(graph))
    } else {
      Ok(None)
    }
  }
  fn get_audio_filter_graph(
    audio_filters: &[AudioFilter],
    audio_decoder: &AudioDecoder,
  ) -> Result<Option<FilterGraph>> {
    let mut graph = FilterGraph::new().map_err(RuntimeError)?;
    let mut filters = vec![];

    for audio_filter in audio_filters {
      let filter = audio_filter
        .as_generic_filter()
        .map_err(RuntimeError)?
        .as_filter()
        .map_err(RuntimeError)?;
      filters.push(graph.add_filter(&filter).map_err(RuntimeError)?);
    }

    if !filters.is_empty() {
      graph
        .add_input_from_audio_decoder("audio_input", audio_decoder)
        .map_err(RuntimeError)?;

      graph
        .add_audio_output("audio_output")
        .map_err(RuntimeError)?;

      let mut filter = filters.remove(0);
      trace!(
        "Connect audio graph input to filter {}...",
        filter.get_label()
      );
      graph
        .connect_input("audio_input", 0, &filter, 0)
        .map_err(RuntimeError)?;

      while !filters.is_empty() {
        let next_filter = filters.remove(0);
        trace!(
          "Connect filter {} to filter {}...",
          filter.get_label(),
          next_filter.get_label()
        );
        graph
          .connect(&filter, 0, &next_filter, 0)
          .map_err(RuntimeError)?;
        filter = next_filter;
      }

      trace!(
        "Connect filter {} to audio graph output...",
        filter.get_label()
      );
      graph
        .connect_output(&filter, 0, "audio_output", 0)
        .map_err(RuntimeError)?;

      graph.validate().map_err(RuntimeError)?;
      Ok(Some(graph))
    } else {
      Ok(None)
    }
  }
}

struct Decoder {
  audio_decoder: Option<AudioDecoder>,
  video_decoder: Option<VideoDecoder>,
  graph: Option<FilterGraph>,
}

impl Decoder {
  fn decode(&self, packet: &Packet) -> std::result::Result<Frame, String> {
    if let Some(audio_decoder) = &self.audio_decoder {
      trace!("[FFmpeg] Send packet to audio decoder");

      let av_frame = unsafe {
        let ret_code = avcodec_send_packet(audio_decoder.codec_context, packet.packet);
        check_result!(ret_code);

        let av_frame = av_frame_alloc();
        let ret_code = avcodec_receive_frame(audio_decoder.codec_context, av_frame);
        check_result!(ret_code);

        let frame = Frame {
          frame: av_frame,
          name: Some("audio_source_1".to_string()),
          index: 1,
        };

        if let Some(graph) = &self.graph {
          if let Ok((audio_frames, _video_frames)) = graph.process(&[frame], &[]) {
            trace!("[FFmpeg] Output graph count {} frames", audio_frames.len());
            let frame = audio_frames.first().unwrap();
            av_frame_clone((*frame).frame)
          } else {
            av_frame
          }
        } else {
          av_frame
        }
      };

      let frame = Frame {
        frame: av_frame,
        name: Some("audio".to_string()),
        index: 1,
      };

      Ok(frame)
    } else if let Some(video_decoder) = &self.video_decoder {
      trace!("[FFmpeg] Send packet to video decoder");

      let av_frame = unsafe {
        let ret_code = avcodec_send_packet(video_decoder.codec_context, packet.packet);
        check_result!(ret_code);

        let av_frame = av_frame_alloc();
        let ret_code = avcodec_receive_frame(video_decoder.codec_context, av_frame);
        check_result!(ret_code);

        let frame = Frame {
          frame: av_frame,
          name: Some("video_source_1".to_string()),
          index: 1,
        };

        if let Some(graph) = &self.graph {
          if let Ok((_audio_frames, video_frames)) = graph.process(&[], &[frame]) {
            trace!("[FFmpeg] Output graph count {} frames", video_frames.len());
            let frame = video_frames.first().unwrap();
            av_frame_clone((*frame).frame)
          } else {
            av_frame
          }
        } else {
          av_frame
        }
      };

      let frame = Frame {
        frame: av_frame,
        name: Some("video".to_string()),
        index: 1,
      };

      Ok(frame)
    } else {
      Err("No audio/video decoder found".to_string())
    }
  }
}
