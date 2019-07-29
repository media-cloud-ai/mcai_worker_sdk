extern crate amqp_worker;
extern crate assert_matches;

use assert_matches::assert_matches;

use amqp_worker::job::*;
use amqp_worker::MessageError;

#[test]
fn test_new_job_empty_message() {
  let message = "";
  let result = Job::new(message);
  assert!(result.is_err());
  let error = result.unwrap_err();
  assert_matches!(error, MessageError::RuntimeError(_));
}

#[test]
fn test_new_job_invalid_message() {
  let message = "{}";
  let result = Job::new(message);
  assert!(result.is_err());
  let error = result.unwrap_err();
  assert_matches!(error, MessageError::RuntimeError(_));
}

#[test]
fn test_new_job_invalid_parameter() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "key":"value" },
    ]
  }"#;
  let result = Job::new(message);
  assert!(result.is_err());
  let error = result.unwrap_err();
  assert_matches!(error, MessageError::RuntimeError(_));
}

#[test]
fn test_new_job() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"string_parameter",
        "type":"string",
        "default":"default_value",
        "value":"real_value" },
      { "id":"boolean_parameter",
        "type":"boolean",
        "default": false,
        "value": true },
      { "id":"integer_parameter",
        "type":"integer",
        "default": 123456,
        "value": 654321 },
      { "id":"credential_parameter",
        "type":"credential",
        "default":"default_credential_key",
        "value":"credential_key" },
      { "id":"array_of_string_parameter",
        "type":"array_of_strings",
        "default": ["default_value"],
        "value": ["real_value"] }
    ]
  }"#;

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  assert_eq!(123, job.job_id);

  let optional_string = job.get_string_parameter("string_parameter");
  assert!(optional_string.is_some());
  let string_value = optional_string.unwrap();
  assert_eq!("real_value".to_string(), string_value);

  let optional_boolean = job.get_boolean_parameter("boolean_parameter");
  assert!(optional_boolean.is_some());
  let boolean_value = optional_boolean.unwrap();
  assert_eq!(true, boolean_value);

  let optional_integer = job.get_integer_parameter("integer_parameter");
  assert!(optional_integer.is_some());
  let integer_value = optional_integer.unwrap();
  assert_eq!(654321, integer_value);

  let optional_credential = job.get_credential_parameter("credential_parameter");
  assert!(optional_credential.is_some());
  let credential_value = optional_credential.unwrap();
  assert_eq!("credential_key", credential_value.key);

  let option_array = job.get_array_of_strings_parameter("array_of_string_parameter");
  assert!(option_array.is_some());
  let array_of_values = option_array.unwrap();
  assert_eq!(1, array_of_values.len());
  assert_eq!("real_value".to_string(), array_of_values[0]);
}

#[test]
fn test_check_requirements() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"requirements",
        "type":"requirements",
        "value": {
          "paths": [
            "./tests/job_test.rs"
          ]
        }
      }
    ]
  }"#;

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  assert_eq!(123, job.job_id);

  let requirement_result = job.check_requirements();
  assert!(requirement_result.is_ok());
}

#[test]
fn test_check_invalid_requirements() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"requirements",
        "type":"requirements",
        "value": {
          "paths": [
            "nonexistent_file"
          ]
        }
      }
    ]
  }"#;

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  assert_eq!(123, job.job_id);

  let requirement_result = job.check_requirements();
  assert!(requirement_result.is_err());
  let error = requirement_result.unwrap_err();
  assert_matches!(error, MessageError::RequirementsError(_));
  if let MessageError::RequirementsError(msg) = error {
    assert_matches!(
      msg.as_str(),
      "Warning: Required file does not exists: \"nonexistent_file\""
    );
  }
}

#[test]
fn test_job_result_from_json() {
  let json = r#"{
    "job_id": 456,
    "status": "completed",
    "parameters": [
      { "id":"string_parameter",
        "type":"string",
        "default":"default_value",
        "value":"real_value" },
      { "id":"boolean_parameter",
        "type":"boolean",
        "default": false,
        "value": true },
      { "id":"integer_parameter",
        "type":"integer",
        "default": 123456,
        "value": 654321 },
      { "id":"credential_parameter",
        "type":"credential",
        "default":"default_credential_key",
        "value":"credential_key" },
      { "id":"array_of_string_parameter",
        "type":"array_of_strings",
        "default": ["default_value"],
        "value": ["real_value"] }
    ]
  }"#;

  let result = serde_json::from_str(json);
  assert!(result.is_ok());
  let job_result: JobResult = result.unwrap();
  assert_eq!(job_result.job_id, 456);
  assert_eq!(job_result.status, JobStatus::Completed);
  assert_eq!(job_result.parameters.len(), 5);

  let optional_string = job_result.get_string_parameter("string_parameter");
  assert!(optional_string.is_some());
  let string_value = optional_string.unwrap();
  assert_eq!("real_value".to_string(), string_value);

  let optional_boolean = job_result.get_boolean_parameter("boolean_parameter");
  assert!(optional_boolean.is_some());
  let boolean_value = optional_boolean.unwrap();
  assert_eq!(true, boolean_value);

  let optional_integer = job_result.get_integer_parameter("integer_parameter");
  assert!(optional_integer.is_some());
  let integer_value = optional_integer.unwrap();
  assert_eq!(654321, integer_value);

  let optional_credential = job_result.get_credential_parameter("credential_parameter");
  assert!(optional_credential.is_some());
  let credential_value = optional_credential.unwrap();
  assert_eq!("credential_key", credential_value.key);

  let option_array = job_result.get_array_of_strings_parameter("array_of_string_parameter");
  assert!(option_array.is_some());
  let array_of_values = option_array.unwrap();
  assert_eq!(1, array_of_values.len());
  assert_eq!("real_value".to_string(), array_of_values[0]);
}

#[test]
fn test_job_result_from_json_without_value() {
  let json = r#"{
    "job_id": 456,
    "status": "completed",
    "parameters": [
      { "id":"string_parameter",
        "type":"string",
        "default":"default_value" },
      { "id":"boolean_parameter",
        "type":"boolean",
        "default": false },
      { "id":"integer_parameter",
        "type":"integer",
        "default": 123456 },
      { "id":"credential_parameter",
        "type":"credential",
        "default":"default_credential_key" },
      { "id":"array_of_string_parameter",
        "type":"array_of_strings",
        "default": ["default_value"] }
    ]
  }"#;

  let result = serde_json::from_str(json);
  assert!(result.is_ok());
  let job_result: JobResult = result.unwrap();
  assert_eq!(job_result.job_id, 456);
  assert_eq!(job_result.status, JobStatus::Completed);
  assert_eq!(job_result.parameters.len(), 5);

  let optional_string = job_result.get_string_parameter("string_parameter");
  assert!(optional_string.is_some());
  let string_value = optional_string.unwrap();
  assert_eq!("default_value".to_string(), string_value);

  let optional_boolean = job_result.get_boolean_parameter("boolean_parameter");
  assert!(optional_boolean.is_some());
  let boolean_value = optional_boolean.unwrap();
  assert_eq!(false, boolean_value);

  let optional_integer = job_result.get_integer_parameter("integer_parameter");
  assert!(optional_integer.is_some());
  let integer_value = optional_integer.unwrap();
  assert_eq!(123456, integer_value);

  let optional_credential = job_result.get_credential_parameter("credential_parameter");
  assert!(optional_credential.is_some());
  let credential_value = optional_credential.unwrap();
  assert_eq!("default_credential_key", credential_value.key);

  let option_array = job_result.get_array_of_strings_parameter("array_of_string_parameter");
  assert!(option_array.is_some());
  let array_of_values = option_array.unwrap();
  assert_eq!(1, array_of_values.len());
  assert_eq!("default_value".to_string(), array_of_values[0]);
}

#[test]
fn test_job_result_from_job() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"string_parameter",
        "type":"string",
        "default":"default_value",
        "value":"real_value" },
      { "id":"boolean_parameter",
        "type":"boolean",
        "default": false,
        "value": true },
      { "id":"integer_parameter",
        "type":"integer",
        "default": 123456,
        "value": 654321 },
      { "id":"credential_parameter",
        "type":"credential",
        "default":"default_credential_key",
        "value":"credential_key" },
      { "id":"array_of_string_parameter",
        "type":"array_of_strings",
        "default": ["default_value"],
        "value": ["real_value"] }
    ]
  }"#;

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  let job_result = JobResult::from(job);
  assert_eq!(job_result.job_id, 123);
  assert_eq!(job_result.status, JobStatus::Unknown);
  assert_eq!(job_result.parameters.len(), 0);
}

#[test]
fn test_job_result_from_job_ref() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"string_parameter",
        "type":"string",
        "default":"default_value",
        "value":"real_value" }
    ]
  }"#;

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  let job_result = JobResult::from(&job);
  assert_eq!(job_result.job_id, 123);
  assert_eq!(job_result.status, JobStatus::Unknown);
  assert_eq!(job_result.parameters.len(), 0);
}

#[test]
fn test_job_result_with_setters() {
  let job_id = 123;
  let mut job_result = JobResult::new(job_id, JobStatus::Unknown, vec![]);
  assert_eq!(job_result.job_id, job_id);
  assert_eq!(job_result.status, JobStatus::Unknown);
  assert_eq!(job_result.parameters.len(), 0);
  job_result = job_result.with_status(JobStatus::Completed);
  assert_eq!(job_result.status, JobStatus::Completed);
  let hello = "Hello!";
  job_result = job_result.with_message(hello.to_string());
  assert_eq!(job_result.status, JobStatus::Completed);

  let optional_string = job_result.get_string_parameter("message");
  assert!(optional_string.is_some());
  let string_value = optional_string.unwrap();
  assert_eq!(hello.to_string(), string_value.to_string());

  // let mut job_result = JobResult::new(job_id, JobStatus::Unknown, vec![]);

  // let e = std::io::Error::new(std::io::ErrorKind::Other, "oh no!");
  // let job_result = job_result.with_error(reqwest::Error::from(e));
}
