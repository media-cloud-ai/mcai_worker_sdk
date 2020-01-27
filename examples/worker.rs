use amqp_worker::job::*;
use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::{MessageError, MessageEvent, ParametersContainer};
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
    semver::Version::new(1, 2, 3)
  }

  fn get_parameters(&self) -> Vec<Parameter> {
    vec![Parameter {
      identifier: "action".to_string(),
      label: "Action".to_string(),
      kind: vec![ParameterType::String],
      required: false,
    }]
  }

  fn process(&self, job: &Job) -> Result<JobResult, MessageError> {
    process_message(job)
  }
}

static WORKER_EVENT: WorkerEvent = WorkerEvent {};

fn main() {
  amqp_worker::start_worker(&WORKER_EVENT);
}

pub fn process_message(job: &Job) -> Result<JobResult, MessageError> {
  match job
    .get_string_parameter("action")
    .unwrap_or("error".to_string())
    .as_str()
  {
    "completed" => Ok(JobResult::new(job.job_id, JobStatus::Completed)),
    action_label => {
      let result = JobResult::new(job.job_id, JobStatus::Error)
        .with_message(&format!("Unknown action named {}", action_label));
      Err(MessageError::ProcessingError(result))
    }
  }
}
