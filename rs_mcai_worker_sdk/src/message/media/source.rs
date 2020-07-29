use crate::{
  error::MessageError::RuntimeError,
  job::JobResult,
  message::media::{media_stream::MediaStream, srt::SrtStream},
  MessageEvent, Result,
};
use ringbuf::RingBuffer;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{
  mpsc,
  mpsc::{Receiver, Sender},
  Arc, Mutex,
};
use std::{cell::RefCell, collections::HashMap, io::Cursor, rc::Rc, thread};

use stainless_ffmpeg::{
  audio_decoder::AudioDecoder,
  filter_graph::FilterGraph,
  format_context::FormatContext,
  frame::Frame,
  order::{filter::Filter, filter_output::FilterOutput, parameters::ParameterValue},
  packet::Packet,
  video_decoder::VideoDecoder,
};
use stainless_ffmpeg_sys::{
  av_frame_alloc, av_frame_clone, avcodec_receive_frame, avcodec_send_packet,
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
}

impl Source {
  pub fn new<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    message_event: Rc<RefCell<ME>>,
    job_result: &JobResult,
    parameters: P,
    source_url: &str,
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
      )?;

      Ok(Source {
        decoders,
        format_context,
        thread: Some(source_thread),
      })
    } else {
      let mut format_context = FormatContext::new(source_url).map_err(RuntimeError)?;
      format_context.open_input().map_err(RuntimeError)?;

      let format_context = Arc::new(Mutex::new(format_context));

      let decoders = Self::get_decoders(
        message_event,
        &job_result.get_str_job_id(),
        parameters,
        format_context.clone(),
      )?;

      Ok(Source {
        decoders,
        format_context,
        thread: None,
      })
    }
  }

  pub fn get_duration(&self) -> Option<f64> {
    if self.thread.is_some() {
      return None;
    }

    self
      .format_context
      .lock()
      .unwrap()
      .get_duration()
      .map(|duration| duration * 25.0)
  }

  pub fn next_frame(&mut self) -> Result<DecodeResult> {
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
  ) -> Result<HashMap<usize, Decoder>> {
    let selected_streams = message_event
      .borrow_mut()
      .init_process(parameters, format_context.clone())?;

    info!(
      target: job_id,
      "Selected stream IDs: {:?}", selected_streams
    );

    let mut decoders = HashMap::<usize, Decoder>::new();
    for selected_stream in &selected_streams {
      // AudioDecoder can decode any codec, not only video
      let audio_decoder = AudioDecoder::new(
        format!("decoder_{}", selected_stream.index),
        &format_context.clone().lock().unwrap(),
        selected_stream.index as isize,
      )
      .map_err(RuntimeError)?;

      let audio_graph = if let Some(audio_configuration) = &selected_stream.audio_configuration {
        let mut graph = FilterGraph::new().unwrap();

        graph
          .add_input_from_audio_decoder("audio_input", &audio_decoder)
          .unwrap();

        graph.add_audio_output("audio_output").unwrap();

        let mut parameters = HashMap::new();

        if !audio_configuration.sample_rates.is_empty() {
          let sample_rates = audio_configuration
            .sample_rates
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("|");

          parameters.insert(
            "sample_rates".to_string(),
            ParameterValue::String(sample_rates),
          );
        }
        if !audio_configuration.channel_layouts.is_empty() {
          parameters.insert(
            "channel_layouts".to_string(),
            ParameterValue::String(audio_configuration.channel_layouts.join("|")),
          );
        }
        if !audio_configuration.sample_formats.is_empty() {
          parameters.insert(
            "sample_fmts".to_string(),
            ParameterValue::String(audio_configuration.sample_formats.join("|")),
          );
        }

        let filter_definition = Filter {
          name: "aformat".to_string(),
          label: Some("aformat_filter".to_string()),
          parameters,
          inputs: None,
          outputs: Some(vec![FilterOutput {
            stream_label: "audio_output".to_string(),
          }]),
        };

        let filter = graph.add_filter(&filter_definition).unwrap();
        graph.connect_input("audio_input", 0, &filter, 0).unwrap();
        graph.connect_output(&filter, 0, "audio_output", 0).unwrap();

        graph.validate().unwrap();

        Some(graph)
      } else {
        None
      };

      println!("{:?}", selected_stream);

      let decoder = Decoder {
        audio_decoder: Some(audio_decoder),
        video_decoder: None,
        graph: audio_graph,
      };

      decoders.insert(selected_stream.index, decoder);
    }
    Ok(decoders)
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
        avcodec_send_packet(audio_decoder.codec_context, packet.packet);

        let av_frame = av_frame_alloc();
        avcodec_receive_frame(audio_decoder.codec_context, av_frame);

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

      return Ok(frame);
    }
    if let Some(video_decoder) = &self.video_decoder {
      return video_decoder.decode(packet);
    }
    unimplemented!();
  }
}
