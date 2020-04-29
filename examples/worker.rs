use mcai_worker_sdk::job::{Job, JobResult, JobStatus};
use mcai_worker_sdk::publish_job_progression;
use mcai_worker_sdk::worker::{Parameter, ParameterType};
use mcai_worker_sdk::{Channel, MessageError, MessageEvent, ParametersContainer};
use semver::Version;

#[derive(Debug)]
struct WorkerEvent {}

impl MessageEvent for WorkerEvent {
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

  fn process(
    &self,
    channel: Option<&Channel>,
    job: &Job,
    job_result: JobResult,
  ) -> Result<JobResult, MessageError> {
    process_message(channel, job, job_result)
  }
}

static WORKER_EVENT: WorkerEvent = WorkerEvent {};

fn main() {
  mcai_worker_sdk::start_worker(&WORKER_EVENT);
}

pub fn process_message(
  channel: Option<&Channel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  publish_job_progression(channel, &job, 50)?;

  match job
    .get_string_parameter("action")
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
