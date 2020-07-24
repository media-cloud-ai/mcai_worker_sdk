#[macro_use]
extern crate serde_derive;

#[cfg(feature = "media")]
use mcai_worker_sdk::{info, FormatContext, Frame, ProcessResult};
use mcai_worker_sdk::{job::JobResult, MessageEvent, Result};
use mcai_worker_sdk::{job::JobStatus, publish_job_progression, McaiChannel, MessageError};
use schemars::JsonSchema;
use semver::Version;
#[cfg(feature = "media")]
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize, JsonSchema)]
struct WorkerParameters {
  action: Option<String>,
  source_path: Option<String>,
  destination_path: Option<String>,
}

#[derive(Debug)]
struct WorkerContext {}

impl MessageEvent<WorkerParameters> for WorkerContext {
  fn get_name(&self) -> String {
    "Example".to_string()
  }

  fn get_short_description(&self) -> String {
    "An example worker".to_string()
  }

  fn get_description(&self) -> String {
    r#"This worker is just an example to demonstrate the API of rs_amqp_worker.
Do no use in production, just for developments."#
      .to_string()
  }

  fn get_version(&self) -> Version {
    Version::new(1, 2, 3)
  }

  fn init(&mut self) -> Result<()> {
    Ok(())
  }

  #[cfg(feature = "media")]
  fn init_process(
    &mut self,
    _parameters: WorkerParameters,
    _format_context: Arc<Mutex<FormatContext>>,
  ) -> Result<Vec<usize>> {
    Ok(vec![0])
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &mut self,
    job_result: JobResult,
    _stream_index: usize,
    frame: Frame,
  ) -> Result<ProcessResult> {
    unsafe {
      let width = (*frame.frame).width;
      let height = (*frame.frame).height;
      let sample_rate = (*frame.frame).sample_rate;
      let channels = (*frame.frame).channels;
      let nb_samples = (*frame.frame).nb_samples;

      if width != 0 && height != 0 {
        info!(
          target: &job_result.get_str_job_id(),
          "PTS: {}, image size: {}x{}",
          frame.get_pts(),
          width,
          height
        );
      } else {
        info!(
          target: &job_result.get_str_job_id(),
          "PTS: {}, sample_rate: {}Hz, channels: {}, nb_samples: {}",
          frame.get_pts(),
          sample_rate,
          channels,
          nb_samples,
        );
      }
    }
    Ok(ProcessResult::new_json(""))
  }

  #[cfg(feature = "media")]
  fn ending_process(&self) -> Result<()> {
    Ok(())
  }

  /// Not called when the "media" feature is enabled
  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: WorkerParameters,
    job_result: JobResult,
  ) -> Result<JobResult> {
    publish_job_progression(channel.clone(), job_result.get_job_id(), 50)?;

    match parameters.action {
      Some(action_label) => match action_label.as_str() {
        "completed" => {
          publish_job_progression(channel, job_result.get_job_id(), 100)?;
          Ok(job_result.with_status(JobStatus::Completed))
        }
        action_label => {
          let result = job_result.with_message(&format!("Unknown action named {}", action_label));
          Err(MessageError::ProcessingError(result))
        }
      },
      None => {
        let result = job_result.with_message(&format!("Unspecified action parameter"));
        Err(MessageError::ProcessingError(result))
      }
    }
  }
}

fn main() {
  let worker_context = WorkerContext {};
  mcai_worker_sdk::start_worker(worker_context);
}
