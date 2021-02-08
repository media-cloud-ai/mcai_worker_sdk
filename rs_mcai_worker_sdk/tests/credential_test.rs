extern crate mcai_worker_sdk;

use mcai_worker_sdk::{
  job::*, parameter::media_segment::MediaSegment, MessageError, ParametersContainer,
};

#[test]
fn test_string_credential_request_value() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": "TEST_CREDENTIAL_VALUE",
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  let credential = job.get_parameter::<String>("test_credential").unwrap();

  assert_eq!("TEST_CREDENTIAL_VALUE".to_string(), credential);
}

#[test]
fn test_integer_credential_request_value_from_string() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": "12345",
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"integer",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  let credential = job.get_parameter::<i64>("test_credential").unwrap();

  assert_eq!(12345, credential);
}

#[test]
fn test_integer_credential_request_value_from_integer() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": 12345,
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"integer",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  let credential = job.get_parameter::<i64>("test_credential").unwrap();

  assert_eq!(12345, credential);
}

#[test]
fn test_boolean_credential_request_value_from_string() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": "true",
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"boolean",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  let credential = job.get_parameter::<bool>("test_credential").unwrap();

  assert_eq!(true, credential);
}

#[test]
fn test_boolean_credential_request_value_from_boolean() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": true,
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"boolean",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  let credential = job.get_parameter::<bool>("test_credential").unwrap();

  assert_eq!(true, credential);
}

#[test]
fn test_array_credential_request_value_from_array_of_strings() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": [
          "value_1",
          "value_2"
        ],
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"array_of_strings",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  let credential: Vec<String> = job.get_parameter("test_credential").unwrap();

  assert_eq!(2, credential.len());
  assert_eq!(&"value_1".to_string(), credential.get(0).unwrap());
  assert_eq!(&"value_2".to_string(), credential.get(1).unwrap());
}

#[test]
fn test_object_credential_request_value_from_media_segments() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": [
          { "start": 0, "end": 999 },
          { "start": 1000, "end": 1999 }
        ],
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"array_of_media_segments",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  let credential: Vec<MediaSegment> = job.get_parameter("test_credential").unwrap();

  assert_eq!(2, credential.len());
  assert_eq!(&MediaSegment::new(0, 999), credential.get(0).unwrap());
  assert_eq!(&MediaSegment::new(1000, 1999), credential.get(1).unwrap());
}

#[test]
fn test_string_credential_request_value_no_session() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions").with_status(404).create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<String>("test_credential"),
    Err(MessageError::ParameterValueError(
      "\"HTTP status client error (404 Not Found) for url (http://127.0.0.1:1234/sessions)\""
        .to_string()
    ))
  );
}

#[test]
fn test_string_credential_request_value_invalid_session() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"bad_token_key": "token"}"#)
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<String>("test_credential"),
    Err(MessageError::ParameterValueError(
      "\"error decoding response body: missing field `access_token` at line 1 column 26\""
        .to_string()
    ))
  );
}

#[test]
fn test_string_credential_request_value_no_credential() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_status(404)
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<String>("test_credential"),
    Err(MessageError::ParameterValueError(
      "\"HTTP status client error (404 Not Found) for url (http://127.0.0.1:1234/credentials/TEST_CREDENTIAL_KEY)\"".to_string()
    ))
  );
}

#[test]
fn test_string_credential_request_value_unauthorized() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_status(401)
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<String>("test_credential"),
    Err(MessageError::ParameterValueError(
      "\"HTTP status client error (401 Unauthorized) for url (http://127.0.0.1:1234/credentials/TEST_CREDENTIAL_KEY)\"".to_string()
    ))
  );
}

#[test]
fn test_string_credential_request_value_invalid_credential() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(r#"{"data": {}}"#)
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<String>("test_credential"),
    Err(MessageError::ParameterValueError(
      "\"error decoding response body: missing field `id` at line 1 column 11\"".to_string()
    ))
  );
}

#[test]
fn test_string_credential_request_value_without_store() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": "TEST_CREDENTIAL_VALUE",
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<String>("test_credential"),
    Ok("TEST_CREDENTIAL_KEY".to_string())
  );
}

#[test]
fn test_string_credential_request_value_with_other_store() {
  std::env::set_var("OTHER_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": "TEST_CREDENTIAL_VALUE",
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "store":"OTHER",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<String>("test_credential"),
    Ok("TEST_CREDENTIAL_VALUE".to_string())
  );
}

#[test]
fn test_string_credential_request_value_with_invalid_store() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/TEST_CREDENTIAL_KEY")
    .with_header("content-type", "application/json")
    .with_body(
      r#"{"data": {
        "id": 666,
        "key": "TEST_CREDENTIAL_KEY",
        "value": "TEST_CREDENTIAL_VALUE",
        "inserted_at": "today"
      }}"#,
    )
    .create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"string",
        "store":"UNKNOWN",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  #[cfg(target_os = "linux")]
  let code = 111;

  #[cfg(target_os = "macos")]
  let code = 61;

  let part_1 = "error sending request for url (http://127.0.0.1:4000/api/sessions): ";
  let part_2 = "error trying to connect: tcp connect error: Connection refused (os error";
  let error_message = format!(r#""{}{} {})""#, part_1, part_2, code);

  assert_eq!(
    job.get_parameter::<String>("test_credential"),
    Err(MessageError::ParameterValueError(error_message))
  );
}
