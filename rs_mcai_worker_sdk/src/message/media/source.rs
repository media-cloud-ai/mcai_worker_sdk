use super::{
  ebu_ttml_live::EbuTtmlLiveDecoder, json::JsonDecoder, media_stream::MediaStream, srt::SrtStream,
  AudioFilter, StreamConfiguration, VideoFilter,
};
use crate::{
  error::MessageError::RuntimeError, job::JobResult, process_frame::ProcessFrame,
  process_result::ProcessResult, MessageError, MessageEvent, Result,
};
use bytes::Buf;
use ringbuf::RingBuffer;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use stainless_ffmpeg::prelude::*;
use stainless_ffmpeg::{
  audio_decoder::AudioDecoder,
  check_result,
  filter_graph::FilterGraph,
  format_context::FormatContext,
  frame::Frame,
  packet::Packet,
  tools::{self, rational::Rational},
  video_decoder::VideoDecoder,
};
use std::{
  collections::HashMap,
  io::Cursor,
  sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
  },
  thread,
};

pub enum DecoderType {
  Audio(AudioDecoder),
  Video(VideoDecoder),
  EbuTtmlLive(EbuTtmlLiveDecoder),
  Json(JsonDecoder),
  Data,
}

pub enum DecodeResult {
  EndOfStream,
  Frame {
    stream_index: usize,
    frame: ProcessFrame,
  },
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
  /// Program duration
  duration: Option<u64>,
  /// Segment duration in ms
  segment_duration: Option<u64>,
  /// Segment entry point in ms
  start_offset: u64,
  /// Time offset into the program
  position: u64,
}

impl Source {
  pub fn new<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    message_event: Arc<Mutex<ME>>,
    job_result: &JobResult,
    parameters: P,
    source_url: &str,
    sender: Arc<Mutex<Sender<ProcessResult>>>,
    start_index_ms: Option<i64>,
    stop_index_ms: Option<i64>,
  ) -> Result<Self> {
    log::info!(target: &job_result.get_str_job_id(), "Opening source: {}", source_url);

    if SrtStream::is_srt_stream(source_url) {
      let (tx, rx): AsyncChannelSenderReceiver = mpsc::channel();
      let cloned_source_url = source_url.to_string();
      let source_thread = thread::spawn(move || {
        let mut srt_stream = SrtStream::open_connection(&cloned_source_url).unwrap();

        let ring_buffer = RingBuffer::<u8>::new(100 * 1024 * 1024);
        let (mut producer, consumer) = ring_buffer.split();

        let (_instant, bytes) = srt_stream
          .receive()
          .expect("Could not get the first bytes from SRT stream.");

        let size = bytes.len();
        log::debug!("Get first {} bytes to define stream format.", size);

        log::trace!("First {} bytes of the SRT stream: {:?}", size, bytes);
        let mut cursor = Cursor::new(bytes);
        let first_byte = cursor.get_u8();

        cursor.set_position(0);
        producer.read_from(&mut cursor, Some(size)).unwrap();

        let (format, threshold) = if first_byte == 0x47 {
          ("mpegts", 1024 * 1024)
        } else {
          ("data", 0)
        };

        let media_stream = MediaStream::new(format, consumer).unwrap();
        log::debug!(
          "Initializing media stream with format {:?}: {:?}",
          format,
          media_stream
        );

        let mut got_stream_info = false;

        loop {
          if let Some((_instant, bytes)) = srt_stream.receive() {
            log::trace!("{:?}", bytes);
            let size = bytes.len();
            let mut cursor = Cursor::new(bytes);

            producer.read_from(&mut cursor, Some(size)).unwrap();

            if !got_stream_info && producer.len() > threshold {
              match media_stream.stream_info() {
                Err(error) => log::error!("{}", error),
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
        start_index_ms,
      )?;

      Ok(Source {
        decoders,
        format_context,
        thread: Some(source_thread),
        duration: None,
        segment_duration: None,
        start_offset: 0,
        position: 0,
      })
    } else {
      let mut format_context = FormatContext::new(source_url).map_err(RuntimeError)?;
      format_context.open_input().map_err(RuntimeError)?;

      let start_offset = start_index_ms.unwrap_or(0);
      let duration = format_context
        .get_duration()
        .map(|seconds| (seconds * 1000.0) as u64);

      let stop: Option<i64> = stop_index_ms.or_else(|| duration.map(|ms| ms as i64));
      let segment_duration = stop.map(|end| (end - start_offset) as u64);

      let format_context = Arc::new(Mutex::new(format_context));

      let decoders = Self::get_decoders(
        message_event,
        &job_result.get_str_job_id(),
        parameters,
        format_context.clone(),
        sender,
        start_index_ms,
      )?;

      Ok(Source {
        decoders,
        format_context,
        thread: None,
        duration,
        segment_duration,
        start_offset: start_offset as u64,
        position: 0,
      })
    }
  }

  pub fn get_stream_time_base(index: isize, format_context: &FormatContext) -> Rational {
    unsafe {
      let time_base = (*format_context.get_stream(index)).time_base;
      Rational::new(time_base.num, time_base.den)
    }
  }

  pub fn seek_in_stream_at(
    stream_index: i32,
    milliseconds: i64,
    format_context: Arc<Mutex<FormatContext>>,
    flag: i32,
  ) -> Result<()> {
    unsafe {
      let format_context = format_context.lock().unwrap();
      let time_base = Self::get_stream_time_base(stream_index as isize, &format_context);
      let time_stamp = Self::get_pts_from_milliseconds(milliseconds, &time_base);
      log::debug!(
        "Seek in source stream {}, at position {} (with time base: {}/{})",
        stream_index,
        time_stamp,
        time_base.num,
        time_base.den
      );

      if av_seek_frame(
        format_context.format_context,
        stream_index,
        time_stamp as i64,
        flag,
      ) != 0
      {
        return Err(MessageError::RuntimeError(format!(
          "Could not seek at expected position into source stream {}.",
          stream_index
        )));
      }
    }
    Ok(())
  }

  pub fn get_pts_from_milliseconds(milliseconds: i64, time_base: &Rational) -> i64 {
    (milliseconds as f64 * time_base.den as f64 / (1000.0 * time_base.num as f64)) as i64
  }

  pub fn get_milliseconds_from_pts(pts: i64, time_base: &Rational) -> u64 {
    (pts as f64 * time_base.num as f64 / time_base.den as f64 * 1000.0) as u64
  }

  pub fn get_start_offset(&self) -> u64 {
    self.start_offset
  }

  pub fn get_duration(&self) -> Option<u64> {
    self.duration
  }

  pub fn get_segment_duration(&self) -> Option<u64> {
    self.segment_duration
  }

  pub fn get_stream_fps(&self, stream_index: usize) -> f64 {
    let stream = self
      .format_context
      .lock()
      .unwrap()
      .get_stream(stream_index as isize);
    unsafe { av_q2d((*stream).avg_frame_rate) }
  }

  pub fn get_first_stream_index(&self) -> usize {
    self.decoders.keys().cloned().min().unwrap_or(0)
  }

  pub fn next_frame(&mut self) -> Result<DecodeResult> {
    let mut format_context = self.format_context.lock().unwrap();

    match format_context.next_packet() {
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

        if let Some(decoder) = self.decoders.get_mut(&stream_index) {
          match decoder.decode(&packet) {
            Ok(Some(frame)) => {
              let time_base = Self::get_stream_time_base(stream_index as isize, &format_context);

              if stream_index == self.get_first_stream_index() {
                self.position = Self::get_milliseconds_from_pts(frame.get_pts(), &time_base);

                // Check whether the end is not reached
                if let Some(segment_duration) = self.segment_duration {
                  if self.position >= self.start_offset + segment_duration {
                    return Ok(DecodeResult::EndOfStream);
                  }
                }
              }

              let start_pts = Self::get_pts_from_milliseconds(self.start_offset as i64, &time_base);

              if frame.get_pts() < start_pts {
                log::trace!(
                  "Need to decode more frames to reach the expected start PTS: {}/{}",
                  frame.get_pts(),
                  start_pts
                );
                return Ok(DecodeResult::WaitMore);
              }

              Ok(DecodeResult::Frame {
                stream_index,
                frame,
              })
            }
            Ok(None) => Ok(DecodeResult::WaitMore),
            Err(message) => {
              if message == "Resource temporarily unavailable"
                || message == "Invalid data found when processing input"
              {
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
    message_event: Arc<Mutex<ME>>,
    job_id: &str,
    parameters: P,
    format_context: Arc<Mutex<FormatContext>>,
    sender: Arc<Mutex<Sender<ProcessResult>>>,
    start_index_ms: Option<i64>,
  ) -> Result<HashMap<usize, Decoder>> {
    let selected_streams =
      message_event
        .lock()
        .unwrap()
        .init_process(parameters, format_context.clone(), sender)?;

    log::info!(
      target: job_id,
      "Selected stream IDs: {:?}",
      selected_streams
    );

    let mut decoders = HashMap::<usize, Decoder>::new();
    for selected_stream in &selected_streams {
      match &selected_stream.configuration {
        StreamConfiguration::Audio(audio_configuration) => {
          // AudioDecoder can decode any codec, not only video
          let audio_decoder = AudioDecoder::new(
            format!("decoder_{}", selected_stream.index),
            &format_context.clone().lock().unwrap(),
            selected_stream.index as isize,
          )
          .map_err(RuntimeError)?;

          let audio_graph =
            Source::get_audio_filter_graph(&audio_configuration.filters, &audio_decoder)?;

          if let Some(ms) = start_index_ms {
            Self::seek_in_stream_at(
              selected_stream.index as i32,
              ms,
              format_context.clone(),
              AVSEEK_FLAG_BACKWARD,
            )?;
          }

          let decoder = Decoder {
            decoder: DecoderType::Audio(audio_decoder),
            graph: audio_graph,
          };

          decoders.insert(selected_stream.index, decoder);
        }
        StreamConfiguration::Image(image_configuration) => {
          let video_decoder = VideoDecoder::new(
            format!("decoder_{}", selected_stream.index),
            &format_context.clone().lock().unwrap(),
            selected_stream.index as isize,
          )
          .map_err(RuntimeError)?;

          let video_graph =
            Source::get_video_filter_graph(&image_configuration.filters, &video_decoder)?;

          let decoder = Decoder {
            decoder: DecoderType::Video(video_decoder),
            graph: video_graph,
          };

          if let Some(ms) = start_index_ms {
            Self::seek_in_stream_at(
              selected_stream.index as i32,
              ms,
              format_context.clone(),
              AVSEEK_FLAG_BACKWARD,
            )?;
          }

          decoders.insert(selected_stream.index, decoder);
        }

        StreamConfiguration::EbuTtmlLive => {
          let ebu_ttml_live_decoder = EbuTtmlLiveDecoder::new();
          let decoder = Decoder {
            decoder: DecoderType::EbuTtmlLive(ebu_ttml_live_decoder),
            graph: None,
          };

          decoders.insert(selected_stream.index, decoder);
        }

        StreamConfiguration::Json => {
          let json_decoder = JsonDecoder::new();
          let decoder = Decoder {
            decoder: DecoderType::Json(json_decoder),
            graph: None,
          };

          decoders.insert(selected_stream.index, decoder);
        }

        StreamConfiguration::Data => {
          let decoder = Decoder {
            decoder: DecoderType::Data,
            graph: None,
          };

          decoders.insert(selected_stream.index, decoder);
        }
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
        .map_err(|error| {
          RuntimeError(format!(
            "Cannot convert video filter to generic filter: {}",
            error
          ))
        })?
        .as_filter()
        .map_err(|error| {
          RuntimeError(format!(
            "Cannot convert generic filter to stainless ffmpeg filter: {}",
            error
          ))
        })?;
      filters.push(graph.add_filter(&filter).map_err(|error| {
        RuntimeError(format!("Cannot add filter {:?} to list: {}", filter, error))
      })?);
    }

    if !filters.is_empty() {
      graph
        .add_input_from_video_decoder("video_input", video_decoder)
        .map_err(RuntimeError)?;
      graph
        .add_video_output("video_output")
        .map_err(RuntimeError)?;

      let mut filter = filters.remove(0);
      log::trace!(
        "Connect video graph input to filter {}...",
        filter.get_label()
      );
      graph
        .connect_input("video_input", 0, &filter, 0)
        .map_err(RuntimeError)?;

      while !filters.is_empty() {
        let next_filter = filters.remove(0);
        log::trace!(
          "Connect filter {} to filter {}...",
          filter.get_label(),
          next_filter.get_label()
        );
        graph
          .connect(&filter, 0, &next_filter, 0)
          .map_err(RuntimeError)?;
        filter = next_filter;
      }

      log::trace!(
        "Connect filter {} to video graph output...",
        filter.get_label()
      );
      graph
        .connect_output(&filter, 0, "video_output", 0)
        .map_err(RuntimeError)?;

      graph.validate().map_err(|error| {
        RuntimeError(format!("Video filter graph validation failed: {}", error))
      })?;
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
        .map_err(|error| {
          RuntimeError(format!(
            "Cannot convert audio filter to generic filter: {}",
            error
          ))
        })?
        .as_filter()
        .map_err(|error| {
          RuntimeError(format!(
            "Cannot convert generic filter to stainless ffmpeg filter: {}",
            error
          ))
        })?;
      filters.push(graph.add_filter(&filter).map_err(|error| {
        RuntimeError(format!("Cannot add filter {:?} to list: {}", filter, error))
      })?);
    }

    if !filters.is_empty() {
      graph
        .add_input_from_audio_decoder("audio_input", audio_decoder)
        .map_err(RuntimeError)?;

      graph
        .add_audio_output("audio_output")
        .map_err(RuntimeError)?;

      let mut filter = filters.remove(0);
      log::trace!(
        "Connect audio graph input to filter {}...",
        filter.get_label()
      );
      graph
        .connect_input("audio_input", 0, &filter, 0)
        .map_err(RuntimeError)?;

      while !filters.is_empty() {
        let next_filter = filters.remove(0);
        log::trace!(
          "Connect filter {} to filter {}...",
          filter.get_label(),
          next_filter.get_label()
        );
        graph
          .connect(&filter, 0, &next_filter, 0)
          .map_err(RuntimeError)?;
        filter = next_filter;
      }

      log::trace!(
        "Connect filter {} to audio graph output...",
        filter.get_label()
      );
      graph
        .connect_output(&filter, 0, "audio_output", 0)
        .map_err(RuntimeError)?;

      graph.validate().map_err(|error| {
        RuntimeError(format!("Audio filter graph validation failed: {}", error))
      })?;
      Ok(Some(graph))
    } else {
      Ok(None)
    }
  }
}

struct Decoder {
  decoder: DecoderType,
  graph: Option<FilterGraph>,
}

impl Decoder {
  fn decode(&mut self, packet: &Packet) -> std::result::Result<Option<ProcessFrame>, String> {
    match &mut self.decoder {
      DecoderType::Audio(audio_decoder) => {
        log::trace!("[FFmpeg] Send packet to audio decoder");

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
              log::trace!("[FFmpeg] Output graph count {} frames", audio_frames.len());
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

        Ok(Some(ProcessFrame::AudioVideo(frame)))
      }

      DecoderType::Video(video_decoder) => {
        log::trace!("[FFmpeg] Send packet to video decoder");

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
              log::trace!("[FFmpeg] Output graph count {} frames", video_frames.len());
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

        Ok(Some(ProcessFrame::AudioVideo(frame)))
      }

      DecoderType::EbuTtmlLive(ebu_ttml_live_decoder) => {
        let result = match ebu_ttml_live_decoder.decode(packet)? {
          Some(ttml_content) => Some(ProcessFrame::EbuTtmlLive(Box::new(ttml_content))),
          None => None,
        };
        Ok(result)
      }

      DecoderType::Json(json_decoder) => {
        let result = match json_decoder.decode(packet)? {
          Some(json_value) => Some(ProcessFrame::Json(Box::new(json_value))),
          None => None,
        };
        Ok(result)
      }

      DecoderType::Data => {
        let data_size = unsafe { (*packet.packet).size as usize };
        let data = unsafe { (*packet.packet).data as *mut u8 };
        let vec_data = unsafe { Vec::from_raw_parts(data, data_size, data_size) };

        Ok(Some(ProcessFrame::Data(vec_data)))
      }
    }
  }
}
