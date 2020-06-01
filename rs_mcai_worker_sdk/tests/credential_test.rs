extern crate mcai_worker_sdk;

use crate::mcai_worker_sdk::ParametersContainer;
use mcai_worker_sdk::job::*;
use mcai_worker_sdk::parameter::ParameterValueError;
use mcai_worker_sdk::Credential;

#[test]
fn test_credential_serialize_deserialize() {
  let credential = Credential {
    value: "TEST_CREDENTIAL_VALUE".to_string(),
  };
  let serialized = serde_json::to_string(&credential).unwrap();
  let deserialized: Credential = serde_json::from_str(&serialized).unwrap();
  assert_eq!(credential, deserialized);
}

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
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Ok(Credential {
      value: "TEST_CREDENTIAL_VALUE".to_string()
    })
  );

  let credential = job.get_parameter::<Credential>("test_credential").unwrap();

  assert_eq!("TEST_CREDENTIAL_VALUE".to_string(), credential.value);
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
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Err(ParameterValueError::new(
      "\"error decoding response body: EOF while parsing a value at line 1 column 0\""
    ))
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
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Err(ParameterValueError::new(
      "\"error decoding response body: missing field `access_token` at line 1 column 26\""
    ))
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
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Err(ParameterValueError::new(
      "\"error decoding response body: EOF while parsing a value at line 1 column 0\""
    ))
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
        "store":"BACKEND",
        "value":"TEST_CREDENTIAL_KEY"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();

  assert_eq!(
    job.get_parameter::<Credential>("test_credential"),
    Err(ParameterValueError::new(
      "\"error decoding response body: missing field `id` at line 1 column 11\""
    ))
  );
}

#[test]
fn test_credential_request_value_without_store() {
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
      value: "TEST_CREDENTIAL_KEY".to_string()
    })
  );

  let credential = job.get_parameter::<Credential>("test_credential").unwrap();

  assert_eq!("TEST_CREDENTIAL_KEY".to_string(), credential.value);
}
