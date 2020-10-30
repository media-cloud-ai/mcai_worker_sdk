extern crate mcai_worker_sdk;

use mcai_worker_sdk::{
  job::*,
  parameter::{media_segment::MediaSegment, MediaSegments},
  MessageError, ParameterValue, ParametersContainer,
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct JsonParamTestStruct {
  key: String,
}

impl ParameterValue for JsonParamTestStruct {
  fn get_type_as_string() -> String {
    "key_test".to_string()
  }
}

#[test]
fn job_result_from_json() {
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
      { "id":"json_parameter",
        "type":"string",
        "default": "{\"key\":\"default\"}",
        "value": "{\"key\":\"value\"}" },
      { "id":"credential_parameter",
        "type":"string",
        "store":"backend",
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

  let result = serde_json::from_str(json);
  assert!(result.is_ok());
  let job_result: JobResult = result.unwrap();
  assert_eq!(job_result.get_job_id(), 456);
  assert_eq!(job_result.get_status(), &JobStatus::Completed);
  assert_eq!(job_result.get_parameters().len(), 7);

  let optional_string = job_result.get_parameter::<String>("string_parameter");
  assert!(optional_string.is_ok());
  assert_eq!("real_value".to_string(), optional_string.unwrap());

  let optional_boolean = job_result.get_parameter::<bool>("boolean_parameter");
  assert!(optional_boolean.is_ok());
  assert_eq!(true, optional_boolean.unwrap());

  let optional_integer = job_result.get_parameter::<i64>("integer_parameter");
  assert!(optional_integer.is_ok());
  assert_eq!(654321, optional_integer.unwrap());

  let optional_json = job_result.get_parameter::<String>("json_parameter");
  assert!(optional_json.is_ok());
  let json_param: JsonParamTestStruct = serde_json::from_str(&optional_json.unwrap()).unwrap();
  assert_eq!("value", &json_param.key);

  let optional_credential = job_result.get_parameter::<String>("credential_parameter");
  assert!(optional_credential.is_err());
  let credential_value = optional_credential.unwrap_err();

  #[cfg(target_os = "linux")]
  let code = 111;

  #[cfg(target_os = "macos")]
  let code = 61;

  let part_1 = "error sending request for url (http://127.0.0.1:4000/api/sessions): ";
  let part_2 = "error trying to connect: tcp connect error: Connection refused (os error";
  let error_message = format!(r#""{}{} {})""#, part_1, part_2, code);
  assert_eq!(
    MessageError::ParameterValueError(error_message),
    credential_value
  );

  let option_array = job_result.get_parameter::<Vec<String>>("array_of_string_parameter");
  assert!(option_array.is_ok());
  let array_of_values = option_array.unwrap();
  assert_eq!(1, array_of_values.len());
  assert_eq!("real_value".to_string(), array_of_values[0]);

  let option_media_segments_array =
    job_result.get_parameter::<Vec<MediaSegment>>("array_of_media_segments_parameter");
  assert!(option_media_segments_array.is_ok());
  let media_segments_array = option_media_segments_array.unwrap();
  assert_eq!(media_segments_array.len(), 1);
  assert_eq!(MediaSegment::new(123, 456), media_segments_array[0]);

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
  reference_map.insert(
    "json_parameter".to_string(),
    "{\"key\":\"value\"}".to_string(),
  );
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
    reference_map.get("json_parameter"),
    map.get("json_parameter")
  );
  assert_eq!(
    reference_map.get("string_parameter"),
    map.get("string_parameter")
  );
  assert_eq!(
    reference_map.get("array_of_media_segments_parameter"),
    map.get("array_of_media_segments")
  );
}

#[test]
fn job_result_from_json_without_value() {
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
      { "id":"json_parameter",
        "type":"string",
        "default": "{\"key\":\"default\"}" },
      { "id":"credential_parameter",
        "type":"string",
        "store":"backend",
        "default":"default_credential_key" },
      { "id":"array_of_string_parameter",
        "type":"array_of_strings",
        "default": ["default_value"] },
      { "id":"array_of_media_segments_parameter",
        "type":"array_of_media_segments",
        "default": [{"start": 123, "end": 456}] }
    ]
  }"#;

  let result = serde_json::from_str(json);
  println!("{:?}", result);
  assert!(result.is_ok());
  let job_result: JobResult = result.unwrap();
  assert_eq!(job_result.get_job_id(), 456);
  assert_eq!(job_result.get_execution_duration(), 0.0);
  assert_eq!(job_result.get_status(), &JobStatus::Completed);
  assert_eq!(job_result.get_parameters().len(), 7);

  let optional_string = job_result.get_parameter::<String>("string_parameter");
  assert!(optional_string.is_ok());
  let string_value = optional_string.unwrap();
  assert_eq!("default_value".to_string(), string_value);

  let optional_boolean = job_result.get_parameter::<bool>("boolean_parameter");
  assert!(optional_boolean.is_ok());
  let boolean_value = optional_boolean.unwrap();
  assert_eq!(false, boolean_value);

  let optional_integer = job_result.get_parameter::<i64>("integer_parameter");
  assert!(optional_integer.is_ok());
  let integer_value = optional_integer.unwrap();
  assert_eq!(123456, integer_value);

  let optional_json = job_result.get_parameter::<String>("json_parameter");
  assert!(optional_json.is_ok());
  let json_param: JsonParamTestStruct = serde_json::from_str(&optional_json.unwrap()).unwrap();
  assert_eq!("default", &json_param.key);

  let optional_credential = job_result.get_parameter::<String>("credential_parameter");
  assert!(optional_credential.is_err());
  let credential_value = optional_credential.unwrap_err();
  #[cfg(target_os = "linux")]
  let code = 111;

  #[cfg(target_os = "macos")]
  let code = 61;

  let part_1 = "error sending request for url (http://127.0.0.1:4000/api/sessions): ";
  let part_2 = "error trying to connect: tcp connect error: Connection refused (os error";
  let error_message = format!(r#""{}{} {})""#, part_1, part_2, code);
  assert_eq!(
    MessageError::ParameterValueError(error_message),
    credential_value
  );

  let option_array = job_result.get_parameter::<Vec<String>>("array_of_string_parameter");
  assert!(option_array.is_ok());
  let array_of_values = option_array.unwrap();
  assert_eq!(1, array_of_values.len());
  assert_eq!("default_value".to_string(), array_of_values[0]);

  let option_media_segments_array =
    job_result.get_parameter::<Vec<MediaSegment>>("array_of_media_segments_parameter");
  assert!(option_media_segments_array.is_ok());
  let media_segments_array = option_media_segments_array.unwrap();
  assert_eq!(media_segments_array.len(), 1);
  assert_eq!(MediaSegment::new(123, 456), media_segments_array[0]);

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
  reference_map.insert(
    "json_parameter".to_string(),
    "{\"key\":\"default\"}".to_string(),
  );
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
    reference_map.get("json_parameter"),
    map.get("json_parameter")
  );
  assert_eq!(
    reference_map.get("string_parameter"),
    map.get("string_parameter")
  );
  assert_eq!(
    reference_map.get("array_of_media_segments_parameter"),
    map.get("array_of_media_segments")
  );
}

#[test]
fn job_result_from_job() {
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
      { "id":"json_parameter",
        "type":"json",
        "default": "{\"key\":\"default\"}",
        "value": "{\"key\":\"value\"}" },
      { "id":"credential_parameter",
        "type":"string",
        "store":"backend",
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
  let job_result = JobResult::from(job);
  assert_eq!(job_result.get_job_id(), 123);
  assert_eq!(job_result.get_status(), &JobStatus::Unknown);
  assert_eq!(job_result.get_parameters().len(), 0);
}

#[test]
fn job_result_from_job_ref() {
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
fn job_result_with_setters() {
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
    Ok(content.to_string()),
    job_result.get_parameter::<String>("message")
  );

  let json_object = JsonParamTestStruct {
    key: "json".to_string(),
  };

  job_result = job_result.with_json("json_param_id", &json_object).unwrap();
  assert_eq!(job_result.get_status(), &JobStatus::Completed);

  let json_param = job_result.get_parameter::<JsonParamTestStruct>("json_param_id");
  assert_eq!(Ok(json_object), json_param);
}

#[test]
fn job_result_with_media_segments() {
  let job_id = 123;
  let mut job_result = JobResult::new(job_id);
  let segments = vec![MediaSegment { start: 0, end: 10 }];
  job_result = job_result.with_json("segments", &segments).unwrap();

  let json_param = job_result.get_parameter::<MediaSegments>("segments");
  assert_eq!(Ok(segments), json_param);
}
