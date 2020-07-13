
use crate::{
  error::MessageError::RuntimeError,
  job::JobResult,
  message::media::{
    srt::SrtStream,
    media_stream::MediaStream,
  },
  MessageError, MessageEvent, Result
};
use ringbuf::RingBuffer;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{
  cell::RefCell,
  collections::HashMap,
  io::Cursor,
  rc::Rc,
};
use stainless_ffmpeg::{
  format_context::FormatContext,
  frame::Frame,
  video_decoder::VideoDecoder
};

pub enum DecodeResult {
  Frame { stream_index: usize, frame: Frame },
  Nothing,
  EndOfStream,
}

pub struct Source {
  decoders: HashMap<usize, VideoDecoder>,
  format_context: FormatContext,
  srt_stream: Option<SrtStream>,
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

    let (srt_stream, format_context) =
      if SrtStream::is_srt_stream(source_url) {
        let mut srt_stream = SrtStream::open_connection(source_url)?;

        let format = "mpegts";

        let ring_buffer = RingBuffer::<u8>::new(100 * 1024 * 1024);
        let (mut producer, consumer) = ring_buffer.split();
        let media_stream = MediaStream::new(format, consumer).map_err(|error| MessageError::from(error, job_result.clone()))?;

        loop {
          if let Some((_instant, bytes)) = srt_stream.receive() {
            trace!("{:?}", bytes);
            let size = bytes.len();
            let mut cursor = Cursor::new(bytes);

            let cloned_job_result = job_result.clone();

            producer.read_from(&mut cursor, Some(size))
              .map_err(|e| MessageError::from(e, cloned_job_result))?;

            if producer.len() > 1024 * 1024 {

              match media_stream.stream_info() {
                Err(error) => error!("{}", error),
                Ok(()) => {
                  println!("GOT STREAM INFO !!!");
                  break;
                }
              }
            }
          }
        }

        let format_context = FormatContext::new(source_url).map_err(RuntimeError)?;
        (Some(srt_stream), format_context)
      } else {
        let mut format_context = FormatContext::new(source_url).map_err(RuntimeError)?;
        format_context.open_input().map_err(RuntimeError)?;

        let selected_streams = message_event
          .borrow_mut()
          .init_process(parameters, &format_context)?;

        info!(
          target: &job_result.get_str_job_id(),
          "Selected stream IDs: {:?}", selected_streams
        );

        for selected_stream in &selected_streams {
          // VideoDecoder can decode any codec, not only video
          let decoder = VideoDecoder::new(
            format!("decoder_{}", selected_stream),
            &format_context,
            *selected_stream as isize,
          )
          .unwrap();
          decoders.insert(*selected_stream, decoder);
        }
        (None, format_context)
      };

    Ok(Source {
      decoders,
      format_context,
      srt_stream,
    })
  }

  pub fn get_duration(&self) -> Option<f64> {
    self
      .format_context
      .get_duration()
      .map(|duration| duration * 25.0)
  }

  pub fn next_frame(&mut self) -> Result<DecodeResult> {
    match self.format_context.next_packet() {
      Err(message) => {
        if message == "End of data stream" || message == "Unable to read next packet" {
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
              println!("{:?}", message);
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
