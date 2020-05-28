extern crate mcai_worker_sdk;

use crate::mcai_worker_sdk::{ParameterValue, ParametersContainer};
use mcai_worker_sdk::job::*;
use mcai_worker_sdk::{Credential, MessageError, Parameter};

#[test]
fn test_credential_request_value() {
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
        "type":"credential",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  let credential = job.get_parameter::<Credential>("test_credential").unwrap();

  assert_eq!(
    Ok("TEST_CREDENTIAL_VALUE".to_string()),
    credential.request_value(&job)
  );
}

#[test]
fn test_credential_request_value_no_session() {
  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions").with_status(404).create();

  let message = r#"{
    "job_id": 123,
    "parameters": [
      { "id":"test_credential",
        "type":"credential",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(mcai_worker_sdk::Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  let credential = job.get_parameter::<Credential>("test_credential").unwrap();

  assert_eq!(
    Err(MessageError::ProcessingError(
      JobResult::new(123)
        .with_status(JobStatus::Error)
        .with_parameters(&mut vec![Parameter {
          id: "message".to_string(),
          kind: String::get_type_as_string(),
          default: None,
          value: serde_json::to_value(
            "error decoding response body: EOF while parsing a value at line 1 column 0"
              .to_string()
          )
          .ok()
        }])
    )),
    credential.request_value(&job)
  );
}

#[test]
fn test_credential_request_value_invalid_session() {
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
        "type":"credential",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  let credential = job.get_parameter::<Credential>("test_credential").unwrap();

  assert_eq!(
    Err(MessageError::ProcessingError(
      JobResult::new(123)
        .with_status(JobStatus::Error)
        .with_parameters(&mut vec![Parameter {
          id: "message".to_string(),
          kind: String::get_type_as_string(),
          default: None,
          value: serde_json::to_value(
            "error decoding response body: missing field `access_token` at line 1 column 26"
              .to_string()
          )
          .ok()
        }])
    )),
    credential.request_value(&job)
  );
}

#[test]
fn test_credential_request_value_no_credential() {
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
        "type":"credential",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  let credential = job.get_parameter::<Credential>("test_credential").unwrap();

  assert_eq!(
    Err(MessageError::ProcessingError(
      JobResult::new(123)
        .with_status(JobStatus::Error)
        .with_parameters(&mut vec![Parameter {
          id: "message".to_string(),
          kind: String::get_type_as_string(),
          default: None,
          value: serde_json::to_value(
            "error decoding response body: EOF while parsing a value at line 1 column 0"
              .to_string()
          )
          .ok()
        }])
    )),
    credential.request_value(&job)
  );
}

#[test]
fn test_credential_request_value_invalid_credential() {
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
        "type":"credential",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      key: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  let credential = job.get_parameter::<Credential>("test_credential").unwrap();

  assert_eq!(
    Err(MessageError::ProcessingError(
      JobResult::new(123)
        .with_status(JobStatus::Error)
        .with_parameters(&mut vec![Parameter {
          id: "message".to_string(),
          kind: String::get_type_as_string(),
          default: None,
          value: serde_json::to_value(
            "error decoding response body: missing field `id` at line 1 column 11".to_string()
          )
          .ok()
        }])
    )),
    credential.request_value(&job)
  );
}
