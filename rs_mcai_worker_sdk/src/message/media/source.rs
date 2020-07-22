
use crate::{
  error::MessageError::RuntimeError,
  job::JobResult,
  message::media::{
    srt::SrtStream,
    media_stream::MediaStream,
  },
  MessageEvent, Result
};
use ringbuf::RingBuffer;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{
  cell::RefCell,
  collections::HashMap,
  io::Cursor,
  rc::Rc,
  thread,
};
use std::sync::{
  Arc,
  Mutex,
  mpsc,
  mpsc::{Sender, Receiver},
};

use stainless_ffmpeg::{
  format_context::FormatContext,
  frame::Frame,
  video_decoder::VideoDecoder
};

pub enum DecodeResult {
  EndOfStream,
  Frame { stream_index: usize, frame: Frame },
  Nothing,
  WaitMore,
}

pub struct Source {
  decoders: HashMap<usize, VideoDecoder>,
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

    let mut decoders = HashMap::<usize, VideoDecoder>::new();

    if SrtStream::is_srt_stream(source_url) {

      let (tx, rx): (Sender<Arc<Mutex<FormatContext>>>, Receiver<Arc<Mutex<FormatContext>>>) = mpsc::channel();
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
                  tx.send(Arc::new(Mutex::new(FormatContext::from(media_stream.format_context)))).unwrap();
                }
              }
            }
          }
        }
      });

      let format_context = rx.recv().unwrap();

      let selected_streams = message_event
        .borrow_mut()
        .init_process(parameters, format_context.clone())?;

      info!(
        target: &job_result.get_str_job_id(),
        "Selected stream IDs: {:?}", selected_streams
      );

      for selected_stream in &selected_streams {
        // VideoDecoder can decode any codec, not only video
        let decoder = VideoDecoder::new(
          format!("decoder_{}", selected_stream),
          &format_context.clone().lock().unwrap(),
          *selected_stream as isize,
        )
        .unwrap();
        decoders.insert(*selected_stream, decoder);
      }

      Ok(Source {
        decoders,
        format_context,
        thread: Some(source_thread),
      })
    } else {
      let mut format_context = FormatContext::new(source_url).map_err(RuntimeError)?;
      format_context.open_input().map_err(RuntimeError)?;

      let format_context = Arc::new(Mutex::new(format_context));

      let selected_streams = message_event
        .borrow_mut()
        .init_process(parameters, format_context.clone())?;

      info!(
        target: &job_result.get_str_job_id(),
        "Selected stream IDs: {:?}", selected_streams
      );

      for selected_stream in &selected_streams {
        // VideoDecoder can decode any codec, not only video
        let decoder = VideoDecoder::new(
          format!("decoder_{}", selected_stream),
          &format_context.clone().lock().unwrap(),
          *selected_stream as isize,
        )
        .unwrap();
        decoders.insert(*selected_stream, decoder);
      }

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
      .lock().unwrap()
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
}
