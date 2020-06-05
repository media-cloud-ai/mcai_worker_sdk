extern crate assert_matches;
extern crate mcai_worker_sdk;

use assert_matches::assert_matches;

use crate::mcai_worker_sdk::ParametersContainer;
use mcai_worker_sdk::job::*;
use mcai_worker_sdk::parameter::media_segment::MediaSegment;
use mcai_worker_sdk::MessageError;

use mcai_worker_sdk::parameter::ParameterValueError;
use mcai_worker_sdk::Credential;
use std::collections::HashMap;

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
        "value": ["real_value"] },
      { "id":"array_of_media_segments_parameter",
        "type":"array_of_media_segments",
        "default": [{"start": 123, "end": 456}],
        "value": [{"start": 123, "end": 456}] }
    ]
  }"#;

  let result = Job::new(message);
  assert!(result.is_ok());
  let job = result.unwrap();
  assert_eq!(job.job_id, 123);

  let optional_string = job.get_parameter::<String>("string_parameter");
  assert!(optional_string.is_ok());
  let string_value = optional_string.unwrap();
  assert_eq!("real_value".to_string(), string_value);

  let optional_boolean = job.get_parameter::<bool>("boolean_parameter");
  assert!(optional_boolean.is_ok());
  let boolean_value = optional_boolean.unwrap();
  assert_eq!(boolean_value, true);

  let optional_integer = job.get_parameter::<i64>("integer_parameter");
  assert!(optional_integer.is_ok());
  let integer_value = optional_integer.unwrap();
  assert_eq!(integer_value, 654321);

  let optional_credential = job.get_parameter::<Credential>("credential_parameter");
  assert!(optional_credential.is_err());
  let credential_value = optional_credential.unwrap_err();
  assert_eq!(ParameterValueError::new("\"error sending request for url (http://127.0.0.1:4000/api/sessions): error trying to connect: tcp connect error: Connection refused (os error 61)\""), credential_value);

  let option_array = job.get_parameter::<Vec<String>>("array_of_string_parameter");
  assert!(option_array.is_ok());
  let array_of_values = option_array.unwrap();
  assert_eq!(array_of_values.len(), 1);
  assert_eq!("real_value".to_string(), array_of_values[0]);

  let option_media_segments_array =
    job.get_parameter::<Vec<MediaSegment>>("array_of_media_segments_parameter");
  assert!(option_media_segments_array.is_ok());
  let media_segments_array = option_media_segments_array.unwrap();
  assert_eq!(media_segments_array.len(), 1);
  assert_eq!(MediaSegment::new(123, 456), media_segments_array[0]);

  let map = job.get_parameters_as_map();
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
  reference_map.insert(
    "array_of_media_segments_parameter".to_string(),
    "[{\"end\":456,\"start\":123}]".to_string(),
  );
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
  assert_eq!(
    reference_map.get("array_of_media_segments_parameter"),
    map.get("array_of_media_segments_parameter")
  );
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
  } else {
    assert!(false);
  }
}
