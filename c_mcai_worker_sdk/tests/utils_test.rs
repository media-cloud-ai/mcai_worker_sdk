use c_mcai_worker_sdk::worker::call_worker_process;
use c_mcai_worker_sdk::{progress, Handler};
use mcai_worker_sdk::job::{Job, JobResult};
use std::os::raw::c_void;

#[test]
pub fn test_c_binding_process() {
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
  let job_result = JobResult::from(job.clone());
  let parameters = job.get_parameters().unwrap();

  let returned_code = call_worker_process(job_result, parameters, None).unwrap();
  assert_eq!(returned_code.get_code(), 0);
  assert_eq!(returned_code.get_message(), "Everything worked well!");
  assert_eq!(
    returned_code.get_output_paths(),
    &vec!["/path/out.mxf".to_string()]
  );
}

#[test]
pub fn test_c_binding_failing_process() {
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
  let job_result = JobResult::from(job.clone());
  let parameters = job.get_parameters().unwrap();

  let returned_code = call_worker_process(job_result, parameters, None).unwrap();
  assert_eq!(returned_code.get_code(), 1);
  assert_eq!(returned_code.get_message(), "Something went wrong...");
  assert!(returned_code.get_output_paths().is_empty());
}

#[test]
pub fn test_c_progress_ptr() {
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
  let parameters = job.get_parameters().ok();

  let handler = Handler {
    job_id: Some(job.job_id),
    parameters,
    channel: None,
  };

  let boxed_handler = Box::new(handler);
  let handler_ptr = Box::into_raw(boxed_handler) as *mut c_void;

  progress(handler_ptr, 25);
  assert!(!handler_ptr.is_null());
}

#[test]
pub fn test_c_progress_with_null_ptr() {
  let null_handler = std::ptr::null_mut();
  progress(null_handler, 50);
  assert!(null_handler.is_null());
}
