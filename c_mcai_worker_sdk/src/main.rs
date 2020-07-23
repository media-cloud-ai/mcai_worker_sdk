mod constants;
mod parameters;
#[cfg(not(feature = "media"))]
mod process_return;
mod worker;

#[macro_use]
extern crate serde_derive;

use crate::parameters::CWorkerParameters;
use crate::worker::*;
#[cfg(not(feature = "media"))]
use mcai_worker_sdk::{debug, job::*, McaiChannel};
use mcai_worker_sdk::{start_worker, MessageEvent, Result, Version};
#[cfg(feature = "media")]
use mcai_worker_sdk::{FormatContext, Frame, ProcessResult};
#[cfg(feature = "media")]
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct CWorkerEvent {}

impl MessageEvent<CWorkerParameters> for CWorkerEvent {
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

  fn init(&mut self) -> Result<()> {
    call_optional_worker_init()
  }

  #[cfg(feature = "media")]
  fn init_process(
    &mut self,
    parameters: CWorkerParameters,
    format_context: Arc<Mutex<FormatContext>>,
  ) -> Result<Vec<usize>> {
    call_worker_init_process(parameters, format_context)
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &mut self,
    str_job_id: &str,
    stream_index: usize,
    frame: Frame,
  ) -> Result<ProcessResult> {
    call_worker_process_frame(str_job_id, stream_index, frame)
  }

  #[cfg(feature = "media")]
  fn ending_process(&self) -> Result<()> {
    call_worker_ending_process()
  }

  #[cfg(not(feature = "media"))]
  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: CWorkerParameters,
    job_result: JobResult,
  ) -> Result<JobResult> {
    debug!("Process job: {}", job_result.get_job_id());
    let process_return = call_worker_process(job_result.clone(), parameters, channel)?;
    debug!("Returned: {:?}", process_return);
    process_return.as_result(job_result)
  }
}

static C_WORKER_EVENT: CWorkerEvent = CWorkerEvent {};

fn main() {
  start_worker(C_WORKER_EVENT.clone());
}

#[test]
pub fn test_c_binding_worker_info() {
  use mcai_worker_sdk::worker::ParameterType;

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

  let parameters = get_worker_parameters();
  assert_eq!(3, parameters.len());

  assert_eq!("my_parameter".to_string(), parameters[0].identifier);
  assert_eq!("My parameter".to_string(), parameters[0].label);
  assert_eq!(1, parameters[0].kind.len());
  assert!(!parameters[0].required);

  let parameter_kind =
    serde_json::to_string(&parameters[0].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&ParameterType::String).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);

  assert_eq!("source_path".to_string(), parameters[1].identifier);
  assert_eq!("Source path".to_string(), parameters[1].label);
  assert_eq!(1, parameters[1].kind.len());
  assert!(parameters[1].required);

  let parameter_kind =
    serde_json::to_string(&parameters[1].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&ParameterType::String).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);

  assert_eq!("destination_path".to_string(), parameters[2].identifier);
  assert_eq!("Destination path".to_string(), parameters[2].label);
  assert_eq!(1, parameters[2].kind.len());
  assert!(parameters[2].required);

  let parameter_kind =
    serde_json::to_string(&parameters[2].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&ParameterType::String).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);
}

#[test]
#[cfg(not(feature = "media"))]
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

  let job = Job::new(message).unwrap();
  let job_result = JobResult::new(job.job_id);
  let parameters = job.get_parameters().unwrap();

  let result = C_WORKER_EVENT.process(None, parameters, job_result);
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
#[cfg(not(feature = "media"))]
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

  let job = Job::new(message).unwrap();
  let job_result = JobResult::new(job.job_id);
  let parameters = job.get_parameters().unwrap();

  let result = C_WORKER_EVENT.process(None, parameters, job_result);
  assert!(result.is_err());
  let _message_error = result.unwrap_err();
}
