use crate::{
  job::{Job, JobResult, JobStatus},
  message::publish_job_progression,
  parameter::container::ParametersContainer,
  AudioFilter, McaiChannel, MessageEvent, Result,
};
use filters::VideoFilter;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use source::DecodeResult;
use std::cell::RefCell;
use std::rc::Rc;

pub mod audio;
pub mod ebu_ttml_live;
pub mod filters;
mod media_stream;
mod output;
pub mod source;
mod srt;
pub mod video;

pub const SOURCE_PATH_PARAMETER: &str = "source_path";
pub const DESTINATION_PATH_PARAMETER: &str = "destination_path";

pub const START_INDEX_PARAMETER: &str = "sdk_start_index";
pub const STOP_INDEX_PARAMETER: &str = "sdk_stop_index";

#[cfg(all(feature = "media"))]
#[derive(Debug, PartialEq)]
pub struct StreamDescriptor {
  index: usize,
  audio_configuration: Option<AudioConfiguration>,
  image_configuration: Option<ImageConfiguration>,
}

impl StreamDescriptor {
  pub fn new_audio(index: usize, filters: Vec<AudioFilter>) -> Self {
    StreamDescriptor {
      index,
      audio_configuration: Some(AudioConfiguration { filters }),
      image_configuration: None,
    }
  }

  pub fn new_video(index: usize, filters: Vec<VideoFilter>) -> Self {
    StreamDescriptor {
      index,
      audio_configuration: None,
      image_configuration: Some(ImageConfiguration { filters }),
    }
  }

  pub fn new_data(index: usize) -> Self {
    StreamDescriptor {
      index,
      audio_configuration: None,
      image_configuration: None,
    }
  }
}

#[cfg(feature = "media")]
#[derive(Debug, PartialEq)]
pub struct AudioConfiguration {
  filters: Vec<AudioFilter>,
}

#[cfg(feature = "media")]
#[derive(Debug, PartialEq)]
pub struct ImageConfiguration {
  filters: Vec<VideoFilter>,
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
  let start_index_ms: Option<i64> = job.get_parameter(START_INDEX_PARAMETER).ok();
  let stop_index_ms: Option<i64> = job.get_parameter(STOP_INDEX_PARAMETER).ok();

  let mut output = output::Output::new(&output_url)?;

  let mut source = source::Source::new(
    message_event.clone(),
    &job_result,
    parameters,
    &source_url,
    output.get_sender(),
    start_index_ms,
    stop_index_ms,
  )?;

  debug!(
    target: &str_job_id,
    "Start to process media (start: {} ms, duration: {})",
    source.get_start_offset(),
    source
      .get_segment_duration()
      .map(|duration| format!("{} ms", duration))
      .unwrap_or_else(|| "unknown".to_string())
  );

  let total_duration = source.get_duration();
  let mut count = 0;
  let mut previous_progress = 0;

  loop {
    match source.next_frame()? {
      DecodeResult::Frame {
        stream_index,
        frame,
      } => {
        if stream_index == source.get_first_stream_index() {
          count += 1;

          if let Some(duration) = total_duration {
            let progress = std::cmp::min((count / duration * 100) as u8, 100);
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
        let job_result = job_result.with_status(JobStatus::Completed);
        return Ok(job_result);
      }
    }
  }
}
