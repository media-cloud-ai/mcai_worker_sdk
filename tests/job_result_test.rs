extern crate amqp_worker;

use crate::amqp_worker::ParametersContainer;
use amqp_worker::job::*;

use std::collections::HashMap;

#[test]
fn test_job_result_from_json() {
  let json = r#"{
    "job_id": 456,
    "status": "completed",
    "destination_paths": [],
    "execution_duration": 0.0,
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
  assert_eq!(job_result.get_job_id(), 456);
  assert_eq!(job_result.get_status(), &JobStatus::Completed);
  assert_eq!(job_result.get_parameters().len(), 5);

  let optional_string = job_result.get_string_parameter("string_parameter");
  assert!(optional_string.is_some());
  assert_eq!("real_value".to_string(), optional_string.unwrap());

  let optional_boolean = job_result.get_boolean_parameter("boolean_parameter");
  assert!(optional_boolean.is_some());
  assert_eq!(true, optional_boolean.unwrap());

  let optional_integer = job_result.get_integer_parameter("integer_parameter");
  assert!(optional_integer.is_some());
  assert_eq!(654321, optional_integer.unwrap());

  let optional_credential = job_result.get_credential_parameter("credential_parameter");
  assert!(optional_credential.is_some());
  let credential_value = optional_credential.unwrap();
  assert_eq!("credential_key", credential_value.key);

  let option_array = job_result.get_array_of_strings_parameter("array_of_string_parameter");
  assert!(option_array.is_some());
  let array_of_values = option_array.unwrap();
  assert_eq!(1, array_of_values.len());
  assert_eq!("real_value".to_string(), array_of_values[0]);

  let map = job_result.get_parameters_as_map();
  let mut reference_map = HashMap::new();
  reference_map.insert(
    "credential_parameter".to_string(),
    "credential_key".to_string(),
  );
  reference_map.insert("boolean_parameter".to_string(), "true".to_string());
  reference_map.insert(
    "array_of_string_parameter".to_string(),
    "[\"real_value\"]".to_string(),
  );
  reference_map.insert("integer_parameter".to_string(), "654321".to_string());
  reference_map.insert("string_parameter".to_string(), "real_value".to_string());
  assert_eq!(
    reference_map.get("credential_parameter"),
    map.get("credential_parameter")
  );
  assert_eq!(
    reference_map.get("boolean_parameter"),
    map.get("boolean_parameter")
  );
  assert_eq!(
    reference_map.get("array_of_string_parameter"),
    map.get("array_of_string_parameter")
  );
  assert_eq!(
    reference_map.get("integer_parameter"),
    map.get("integer_parameter")
  );
  assert_eq!(
    reference_map.get("string_parameter"),
    map.get("string_parameter")
  );
}

#[test]
fn test_job_result_from_json_without_value() {
  let json = r#"{
    "job_id": 456,
    "status": "completed",
    "destination_paths": [],
    "execution_duration": 0.0,
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
  println!("{:?}", result);
  assert!(result.is_ok());
  let job_result: JobResult = result.unwrap();
  assert_eq!(job_result.get_job_id(), 456);
  assert_eq!(job_result.get_execution_duration(), 0.0);
  assert_eq!(job_result.get_status(), &JobStatus::Completed);
  assert_eq!(job_result.get_parameters().len(), 5);

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

  let map = job_result.get_parameters_as_map();
  let mut reference_map = HashMap::new();
  reference_map.insert(
    "credential_parameter".to_string(),
    "default_credential_key".to_string(),
  );
  reference_map.insert("boolean_parameter".to_string(), "false".to_string());
  reference_map.insert(
    "array_of_string_parameter".to_string(),
    "[\"default_value\"]".to_string(),
  );
  reference_map.insert("integer_parameter".to_string(), "123456".to_string());
  reference_map.insert("string_parameter".to_string(), "default_value".to_string());
  assert_eq!(
    reference_map.get("credential_parameter"),
    map.get("credential_parameter")
  );
  assert_eq!(
    reference_map.get("boolean_parameter"),
    map.get("boolean_parameter")
  );
  assert_eq!(
    reference_map.get("array_of_string_parameter"),
    map.get("array_of_string_parameter")
  );
  assert_eq!(
    reference_map.get("integer_parameter"),
    map.get("integer_parameter")
  );
  assert_eq!(
    reference_map.get("string_parameter"),
    map.get("string_parameter")
  );
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
  assert_eq!(job_result.get_job_id(), 123);
  assert_eq!(job_result.get_status(), &JobStatus::Unknown);
  assert_eq!(job_result.get_parameters().len(), 0);
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

  let job = Job::new(message).unwrap();
  let job_result = JobResult::from(&job);
  assert_eq!(job_result.get_job_id(), 123);
  assert_eq!(job_result.get_status(), &JobStatus::Unknown);
  assert_eq!(job_result.get_parameters().len(), 0);
}

#[test]
fn test_job_result_with_setters() {
  let job_id = 123;
  let mut job_result = JobResult::new(job_id);
  assert_eq!(job_result.get_job_id(), job_id);
  assert_eq!(job_result.get_status(), &JobStatus::Unknown);
  assert_eq!(job_result.get_parameters().len(), 0);
  job_result = job_result.with_status(JobStatus::Completed);
  assert_eq!(job_result.get_status(), &JobStatus::Completed);
  let content = "Hello!";

  job_result = job_result.with_message(content);
  assert_eq!(job_result.get_status(), &JobStatus::Completed);

  assert_eq!(
    Some(content.to_string()),
    job_result.get_string_parameter("message")
  );
}
