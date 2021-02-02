use c_mcai_worker_sdk::get_worker_parameters;
use c_mcai_worker_sdk::worker::CWorkerEvent;
use mcai_worker_sdk::prelude::*;

#[test]
pub fn test_c_binding_worker_info() {
  let worker_event = CWorkerEvent::default();
  let name = worker_event.get_name();
  let short_description = worker_event.get_short_description();
  let description = worker_event.get_description();
  let version = worker_event.get_version().to_string();

  assert_eq!(name, "my_c_worker".to_string());
  assert_eq!(short_description, "My C Worker".to_string());
  assert_eq!(
    description,
    "This is my long description \nover multilines".to_string()
  );
  assert_eq!(version, "0.1.0".to_string());

  let parameters = get_worker_parameters();
  assert_eq!(2, parameters.len());

  assert_eq!("my_parameter".to_string(), parameters[0].identifier);
  assert_eq!("My parameter".to_string(), parameters[0].label);
  assert_eq!(1, parameters[0].kind.len());
  assert!(!parameters[0].required);

  let parameter_kind =
    serde_json::to_string(&parameters[0].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&WorkerParameterType::String).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);

  assert_eq!("path".to_string(), parameters[1].identifier);
  assert_eq!("My path parameter".to_string(), parameters[1].label);
  assert_eq!(1, parameters[1].kind.len());
  assert!(parameters[1].required);

  let parameter_kind =
    serde_json::to_string(&parameters[1].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&WorkerParameterType::String).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);
}

#[test]
pub fn test_init() {
  let mut c_worker_event = CWorkerEvent::default();
  let result = c_worker_event.init();
  assert!(result.is_ok());
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

  let job = Job::new(message).unwrap();
  let job_result = JobResult::new(job.job_id);
  let parameters = job.get_parameters().unwrap();

  let result = CWorkerEvent::default().process(None, parameters, job_result);
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

  let job = Job::new(message).unwrap();
  let job_result = JobResult::new(job.job_id);
  let parameters = job.get_parameters().unwrap();

  let result = CWorkerEvent::default().process(None, parameters, job_result);
  assert!(result.is_err());
  let _message_error = result.unwrap_err();
}
