use mcai_worker_sdk::job::{Job, JobResult, JobStatus};
use mcai_worker_sdk::worker::{Parameter, ParameterType};
#[cfg(feature = "media")]
use mcai_worker_sdk::{info, FormatContext, Frame};
use mcai_worker_sdk::{publish_job_progression, McaiChannel};
use mcai_worker_sdk::{MessageError, MessageEvent, ParametersContainer};
use semver::Version;

#[derive(Debug)]
struct WorkerContext {}

impl MessageEvent for WorkerContext {
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

  fn get_parameters(&self) -> Vec<Parameter> {
    vec![Parameter {
      identifier: "action".to_string(),
      label: "Action".to_string(),
      kind: vec![ParameterType::String],
      required: false,
    }]
  }

  #[cfg(not(feature = "media"))]
  fn process(
    &self,
    channel: Option<McaiChannel>,
    job: &Job,
    job_result: JobResult,
  ) -> Result<JobResult, MessageError> {
    process_message(channel, job, job_result)
  }

  #[cfg(feature = "media")]
  fn init_process(&self, format_context: &FormatContext) -> Result<Vec<usize>, MessageError> {
    Ok(vec![1])
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &self,
    job_id: &str,
    stream_index: usize,
    frame: Frame,
  ) -> Result<(), MessageError> {
    unsafe {
      let width = (*frame.frame).width;
      let height = (*frame.frame).height;
      let sample_rate = (*frame.frame).sample_rate;
      let channels = (*frame.frame).channels;
      let nb_samples = (*frame.frame).nb_samples;

      if width != 0 && height != 0 {
        info!(
          target: job_id,
          "PTS: {}, image size: {}x{}",
          frame.get_pts(),
          width,
          height
        );
      } else {
        info!(
          target: job_id,
          "PTS: {}, sample_rate: {}Hz, channels: {}, nb_samples: {}",
          frame.get_pts(),
          sample_rate,
          channels,
          nb_samples,
        );
      }
    }
    Ok(())
  }
}

fn main() {
  let worker_context = WorkerContext {};
  mcai_worker_sdk::start_worker(worker_context);
}

pub fn process_message(
  channel: Option<McaiChannel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  publish_job_progression(channel.clone(), &job, 50)?;

  match job
    .get_parameter::<String>("action")
    .unwrap_or("error".to_string())
    .as_str()
  {
    "completed" => {
      publish_job_progression(channel, &job, 100)?;
      Ok(job_result.with_status(JobStatus::Completed))
    }
    action_label => {
      let result = job_result.with_message(&format!("Unknown action named {}", action_label));
      Err(MessageError::ProcessingError(result))
    }
  }
}
