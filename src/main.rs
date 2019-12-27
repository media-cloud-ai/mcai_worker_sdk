extern crate libc;
extern crate libloading;
#[macro_use]
extern crate log;

mod constants;
mod process_return;
mod worker;

use amqp_worker::job::*;
use amqp_worker::start_worker;
use amqp_worker::worker::Parameter;
use amqp_worker::MessageError;
use amqp_worker::MessageEvent;
use semver::Version;

use crate::worker::*;

#[derive(Debug)]
struct CWorkerEvent {}

impl MessageEvent for CWorkerEvent {
  fn get_name(&self) -> String {
    get_worker_function_string_value(constants::GET_NAME_FUNCTION)
  }

  fn get_short_description(&self) -> String {
    get_worker_function_string_value(constants::GET_SHORT_DESCRIPTION_FUNCTION)
  }

  fn get_description(&self) -> String {
    get_worker_function_string_value(constants::GET_DESCRIPTION_FUNCTION)
  }

  fn get_version(&self) -> Version {
    let version = get_worker_function_string_value(constants::GET_VERSION_FUNCTION);
    Version::parse(&version).unwrap_or_else(|_| {
      panic!(
        "unable to parse version {} (please use SemVer format)",
        version
      )
    })
  }

  fn get_parameters(&self) -> Vec<Parameter> {
    get_worker_parameters()
  }

  fn process(&self, message: &str) -> Result<JobResult, MessageError> {
    let job = Job::new(message)?;
    debug!("received message: {:?}", job);

    job.check_requirements()?;

    let job_id = job.job_id;
    debug!("Process job: {:?}", job_id);
    let process_return = call_worker_process(job);
    debug!("Returned: {:?}", process_return);
    process_return.as_result(job_id)
  }
}

static C_WORKER_EVENT: CWorkerEvent = CWorkerEvent {};

fn main() {
  start_worker(&C_WORKER_EVENT);
}

#[test]
pub fn test_c_binding_worker_info() {
  use amqp_worker::worker::ParameterType;

  let name = C_WORKER_EVENT.get_name();
  let short_description = C_WORKER_EVENT.get_short_description();
  let description = C_WORKER_EVENT.get_description();
  let version = C_WORKER_EVENT.get_version().to_string();

  assert_eq!(name, "my_c_worker".to_string());
  assert_eq!(short_description, "My C Worker".to_string());
  assert_eq!(
    description,
    "This is my long description \nover multilines".to_string()
  );
  assert_eq!(version, "0.1.0".to_string());

  let parameters = C_WORKER_EVENT.get_parameters();
  assert_eq!(1, parameters.len());
  let expected_parameter = Parameter {
    identifier: "my_parameter".to_string(),
    label: "My parameter".to_string(),
    kind: vec![ParameterType::String],
    required: true,
  };
  assert_eq!(expected_parameter.identifier, parameters[0].identifier);
  assert_eq!(expected_parameter.label, parameters[0].label);
  assert_eq!(expected_parameter.kind.len(), parameters[0].kind.len());

  let parameter_kind =
    serde_json::to_string(&parameters[0].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&expected_parameter.kind[0]).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);
  assert_eq!(expected_parameter.required, parameters[0].required);
}

#[test]
pub fn test_process() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      {
        "id": "path",
        "type": "string",
        "value": "/path/to/file"
      }
    ]
  }"#;

  let result = C_WORKER_EVENT.process(message);
  assert!(result.is_ok());
  let job_result = result.unwrap();
  assert_eq!(job_result.get_job_id(), 123);
  assert_eq!(job_result.get_status(), &JobStatus::Completed);
  assert_eq!(
    job_result.get_destination_paths(),
    &vec!["/path/out.mxf".to_string()]
  );
}

#[test]
pub fn test_failing_process() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      {
        "id": "not_the_expected_path_parameter",
        "type": "string",
        "value": "value"
      }
    ]
  }"#;

  let result = C_WORKER_EVENT.process(message);
  assert!(result.is_err());
  let _message_error = result.unwrap_err();
}
