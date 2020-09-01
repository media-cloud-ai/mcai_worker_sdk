use crate::message::media::video::Scaling;
use crate::{
  job::{Job, JobResult},
  message::media::video::RegionOfInterest,
  message::publish_job_progression,
  parameter::container::ParametersContainer,
  McaiChannel, MessageEvent, Result,
};
#[cfg(all(feature = "media", feature = "python"))]
use dict_derive::{FromPyObject, IntoPyObject};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use source::DecodeResult;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod media_stream;
mod output;
mod source;
mod srt;
pub mod ttml;
pub mod video;

pub const SOURCE_PATH_PARAMETER: &str = "source_path";
pub const DESTINATION_PATH_PARAMETER: &str = "destination_path";

#[cfg(all(feature = "media"))]
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct StreamDescriptor {
  index: usize,
  audio_configuration: Option<AudioConfiguration>,
  image_configuration: Option<ImageConfiguration>,
}

impl StreamDescriptor {
  pub fn new_audio(
    index: usize,
    channel_layouts: Vec<String>,
    sample_formats: Vec<String>,
    sample_rates: Vec<usize>,
  ) -> Self {
    StreamDescriptor {
      index,
      audio_configuration: Some(AudioConfiguration {
        channel_layouts,
        sample_formats,
        sample_rates,
      }),
      image_configuration: None,
    }
  }

  pub fn new_video(
    index: usize,
    region_of_interest: Option<RegionOfInterest>,
    resize: Option<Scaling>,
    format_filter_parameters: Option<HashMap<String, String>>,
  ) -> Self {
    StreamDescriptor {
      index,
      audio_configuration: None,
      image_configuration: Some(ImageConfiguration {
        region_of_interest,
        resize,
        format_filter_parameters,
      }),
    }
  }
}

#[cfg(feature = "media")]
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct AudioConfiguration {
  channel_layouts: Vec<String>,
  sample_formats: Vec<String>,
  sample_rates: Vec<usize>,
}

#[cfg(feature = "media")]
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct ImageConfiguration {
  region_of_interest: Option<RegionOfInterest>,
  resize: Option<Scaling>,
  format_filter_parameters: Option<HashMap<String, String>>,
}

pub fn process<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Rc<RefCell<ME>>,
  channel: Option<McaiChannel>,
  job: &Job,
  parameters: P,
  job_result: JobResult,
) -> Result<JobResult> {
  let str_job_id = job.job_id.to_string();

  let source_url: String = job.get_parameter(SOURCE_PATH_PARAMETER)?;
  let output_url: String = job.get_parameter(DESTINATION_PATH_PARAMETER)?;

  let mut output = output::Output::new(&output_url)?;

  let mut source = source::Source::new(
    message_event.clone(),
    &job_result,
    parameters,
    &source_url,
    output.get_sender(),
  )?;

  info!(target: &str_job_id, "Start to process media");

  let total_duration = source.get_duration();
  let mut count = 0;
  let mut previous_progress = 0;

  loop {
    match source.next_frame()? {
      DecodeResult::Frame {
        stream_index,
        frame,
      } => {
        if stream_index == 0 {
          count += 1;

          if let Some(duration) = total_duration {
            let progress = std::cmp::min((count as f64 / duration * 100.0) as u8, 100);
            if progress > previous_progress {
              publish_job_progression(channel.clone(), job.job_id, progress)?;
              previous_progress = progress;
            }
          }
        }

        trace!(target: &job_result.get_str_job_id(), "Process frame {}", count);
        let result =
          message_event
            .borrow_mut()
            .process_frame(job_result.clone(), stream_index, frame)?;

        output.push(result);
      }
      DecodeResult::WaitMore => {}
      DecodeResult::Nothing => {}
      DecodeResult::EndOfStream => {
        message_event.borrow_mut().ending_process()?;

        output.complete()?;
        return Ok(job_result);
      }
    }
  }
}
