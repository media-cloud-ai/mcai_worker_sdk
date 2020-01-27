use amqp_worker::job::*;
use amqp_worker::parameter::container::ParametersContainer;
use amqp_worker::parse_and_process_message;
use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::{MessageError, MessageEvent};
use semver::Version;
use std::env;
use std::path::Path;

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

  fn process(&self, job: &Job, job_result: JobResult) -> Result<JobResult, MessageError> {
    process_message(job, job_result)
  }
}

pub fn process_message(job: &Job, job_result: JobResult) -> Result<JobResult, MessageError> {
  match job
    .get_string_parameter("action")
    .unwrap_or("error".to_string())
    .as_str()
  {
    "completed" => Ok(job_result.with_status(JobStatus::Completed)),
    action_label => {
      let result = job_result.with_message(&format!("Unknown action named {}", action_label));
      Err(MessageError::ProcessingError(result))
    }
  }
}

static WORKER_EVENT: WorkerEvent = WorkerEvent {};

fn main() {
  let args = env::args();
  if args.len() == 2 {
    if let Some(path_or_json) = args.last() {
      let path = Path::new(&path_or_json);

      let result = if path.exists() {
        let message = std::fs::read_to_string(&path_or_json)
          .expect(&format!("unable to read content of: {}", path_or_json));

        parse_and_process_message(&WORKER_EVENT, &message, None)
      } else {
        parse_and_process_message(&WORKER_EVENT, &path_or_json, None)
      };

      println!("{:?}", result);
    } else {
      println!("Unable to get last parameters");
    }
  } else {
    println!("Missing 2nd parameter, pass raw json data or path to json file");
  }
}
