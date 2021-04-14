pub mod audio;
pub mod ebu_ttml_live;
pub mod filters;
pub mod json;
mod media_stream;
pub mod output;
pub mod source;
mod srt;
pub mod video;

use crate::{
  job::{Job, JobResult, JobStatus},
  message::{media::output::Output, media::source::Source},
  parameter::container::ParametersContainer,
  process_frame::ProcessFrame,
  MessageEvent, Result,
};
use audio::AudioFormat;
use filters::{AudioFilter, VideoFilter};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};
use video::{RegionOfInterest, Scaling, VideoFormat};

pub const SOURCE_PATH_PARAMETER: &str = "source_path";
pub const DESTINATION_PATH_PARAMETER: &str = "destination_path";

pub const START_INDEX_PARAMETER: &str = "sdk_start_index";
pub const STOP_INDEX_PARAMETER: &str = "sdk_stop_index";

#[cfg(all(feature = "media"))]
#[derive(Debug, PartialEq)]
pub enum StreamConfiguration {
  Audio(AudioConfiguration),
  Image(ImageConfiguration),
  EbuTtmlLive,
  Json,
  Data,
}

#[cfg(all(feature = "media"))]
#[derive(Debug, PartialEq)]
pub struct StreamDescriptor {
  index: usize,
  configuration: StreamConfiguration,
}

impl StreamDescriptor {
  pub fn new_audio(index: usize, filters: Vec<AudioFilter>) -> Self {
    StreamDescriptor {
      index,
      configuration: StreamConfiguration::Audio(AudioConfiguration { filters }),
    }
  }

  pub fn new_video(index: usize, filters: Vec<VideoFilter>) -> Self {
    StreamDescriptor {
      index,
      configuration: StreamConfiguration::Image(ImageConfiguration { filters }),
    }
  }

  pub fn new_ebu_ttml_live(index: usize) -> Self {
    StreamDescriptor {
      index,
      configuration: StreamConfiguration::EbuTtmlLive,
    }
  }

  pub fn new_json(index: usize) -> Self {
    StreamDescriptor {
      index,
      configuration: StreamConfiguration::Json,
    }
  }

  pub fn new_data(index: usize) -> Self {
    StreamDescriptor {
      index,
      configuration: StreamConfiguration::Data,
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

pub fn initialize_process<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Arc<Mutex<ME>>,
  job: &Job,
) -> Result<(Source, Output)> {
  let job_result = JobResult::new(job.job_id);
  let parameters = job.get_parameters()?;

  let source_url: String = job.get_parameter(SOURCE_PATH_PARAMETER)?;
  let output_url: String = job.get_parameter(DESTINATION_PATH_PARAMETER)?;
  let start_index_ms: Option<i64> = job.get_parameter(START_INDEX_PARAMETER).ok();
  let stop_index_ms: Option<i64> = job.get_parameter(STOP_INDEX_PARAMETER).ok();

  let output = output::Output::new(&output_url)?;

  let source = source::Source::new(
    message_event,
    &job_result,
    parameters,
    &source_url,
    output.get_sender(),
    start_index_ms,
    stop_index_ms,
  )?;

  Ok((source, output))
}

pub fn finish_process<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Arc<Mutex<ME>>,
  output: &mut Output,
  job_result: JobResult,
) -> Result<JobResult> {
  message_event.lock().unwrap().ending_process()?;

  output.complete()?;
  let job_result = job_result.with_status(JobStatus::Completed);
  Ok(job_result)
}

pub fn process_frame<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Arc<Mutex<ME>>,
  output: &mut Output,
  job_result: JobResult,
  stream_index: usize,
  frame: ProcessFrame,
) -> Result<()> {
  let result = message_event
    .lock()
    .unwrap()
    .process_frame(job_result, stream_index, frame)?;

  output.push(result);

  Ok(())
}
