use crate::{error::MessageError::RuntimeError, job::Job, MessageError, MessageEvent};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use stainless_ffmpeg::{format_context::FormatContext, frame::Frame, video_decoder::VideoDecoder};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub enum DecodeResult {
  Frame { stream_index: usize, frame: Frame },
  Nothing,
  EndOfStream,
}

pub struct Source {
  decoders: HashMap<usize, VideoDecoder>,
  format_context: FormatContext,
}

impl Source {
  pub fn new<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    message_event: Rc<RefCell<ME>>,
    job: &Job,
    source_url: &str,
  ) -> Result<Self, MessageError> {
    info!(target: &job.job_id.to_string(), "Openning source: {}", source_url);

    let mut format_context = FormatContext::new(source_url).map_err(RuntimeError)?;
    format_context.open_input().map_err(RuntimeError)?;

    let str_job_id = job.job_id.to_string();

    let selected_streams = message_event
      .borrow_mut()
      .init_process(job, &format_context)?;

    info!(
      target: &str_job_id,
      "Selected stream IDs: {:?}", selected_streams
    );

    let mut decoders = HashMap::<usize, VideoDecoder>::new();

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

    Ok(Source {
      decoders,
      format_context,
    })
  }

  pub fn get_duration(&self) -> Option<f64> {
    self
      .format_context
      .get_duration()
      .map(|duration| duration * 25.0)
  }

  pub fn next_frame(&mut self) -> Result<DecodeResult, MessageError> {
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
            Ok(frame) => {
              return Ok(DecodeResult::Frame {
                stream_index,
                frame,
              });
            }
            Err(message) => {
              println!("{:?}", message);
              if message == "Resource temporarily unavailable" {
                return Ok(DecodeResult::Nothing);
              }
              return Err(RuntimeError(message));
            }
          }
        } else {
          Ok(DecodeResult::Nothing)
        }
      }
    }
  }
}
