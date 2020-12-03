use std::cell::RefCell;
use std::rc::Rc;

use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use filters::VideoFilter;
use source::DecodeResult;

use crate::worker::context::WorkerContext;
use crate::{
  job::{Job, JobResult, JobStatus},
  message::publish_job_progression,
  parameter::container::ParametersContainer,
  AudioFilter, McaiChannel, MessageError, MessageEvent, Result,
};
use std::sync::{Arc, Mutex};

pub mod audio;
pub mod ebu_ttml_live;
pub mod filters;
mod media_stream;
pub mod output;
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

pub fn initialize_process<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Rc<RefCell<ME>>,
  channel: Option<McaiChannel>,
  worker_context: &mut WorkerContext,
  job: &Job,
) -> Result<()> {
  job.check_requirements()?;
  let parameters: P = job.get_parameters()?;

  publish_job_progression(channel.clone(), job.job_id, 0)?;

  let job_result = JobResult::new(job.job_id);

  initialize_process_with_parameters(message_event, worker_context, job, parameters, &job_result)
}

pub fn initialize_process_with_parameters<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Rc<RefCell<ME>>,
  worker_context: &mut WorkerContext,
  job: &Job,
  parameters: P,
  job_result: &JobResult,
) -> Result<()> {
  let source_url: String = job.get_parameter(SOURCE_PATH_PARAMETER)?;
  let output_url: String = job.get_parameter(DESTINATION_PATH_PARAMETER)?;
  let start_index_ms: Option<i64> = job.get_parameter(START_INDEX_PARAMETER).ok();
  let stop_index_ms: Option<i64> = job.get_parameter(STOP_INDEX_PARAMETER).ok();

  let output = output::Output::new(&output_url)?;

  let source = source::Source::new(
    message_event.clone(),
    job_result,
    parameters,
    &source_url,
    output.get_sender(),
    start_index_ms,
    stop_index_ms,
  )?;

  worker_context.source = Some(Arc::new(Mutex::new(source)));
  worker_context.output = Some(Arc::new(Mutex::new(output)));

  Ok(())
}

pub fn launch_process<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Rc<RefCell<ME>>,
  channel: Option<McaiChannel>,
  worker_context: &mut WorkerContext,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult> {
  let str_job_id = job.job_id.to_string();

  let source_ref = worker_context.source.as_ref().ok_or_else(|| {
    MessageError::RuntimeError(format!(
      "The process must be initialized before starting: a source is missing."
    ))
  })?;
  let output_ref = worker_context.output.as_ref().ok_or_else(|| {
    MessageError::RuntimeError(format!(
      "The process must be initialized before starting: an output is missing."
    ))
  })?;

  let mut source = source_ref.lock().unwrap();
  let mut output = output_ref.lock().unwrap();

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

pub fn stop_process<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Rc<RefCell<ME>>,
  worker_context: &WorkerContext,
  job_result: JobResult,
) -> Result<JobResult> {
  message_event.borrow_mut().ending_process()?;

  if let Some(output_ref) = &worker_context.output {
    output_ref.lock().unwrap().complete()?;
  } else {
    warn!("Try to end a process that has already been completed.")
  }

  let job_result = job_result.with_status(JobStatus::Completed);
  Ok(job_result)
}

pub fn process<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Rc<RefCell<ME>>,
  channel: Option<McaiChannel>,
  job: &Job,
  parameters: P,
  job_result: JobResult,
) -> Result<JobResult> {
  // TODO
  let mut worker_context = WorkerContext::new(None);

  initialize_process_with_parameters(
    message_event.clone(),
    &mut worker_context,
    job,
    parameters,
    &job_result,
  )?;

  launch_process(message_event, channel, &mut worker_context, job, job_result)
}
