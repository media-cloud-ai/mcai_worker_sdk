use crate::{
  config::*,
  job::{Session, SessionBody, SessionResponseBody, ValueResponseBody},
};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_json::Value;

#[deprecated(
  since = "0.10.4",
  note = "Please use the `store` field in Parameter instead"
)]
#[derive(Debug, PartialEq)]
pub struct Credential {
  pub value: String,
}

#[cfg_attr(feature = "cargo-clippy", allow(deprecated))]
impl Serialize for Credential {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.value)
  }
}

struct CredentialVisitor;

#[cfg_attr(feature = "cargo-clippy", allow(deprecated))]
impl<'de> Visitor<'de> for CredentialVisitor {
  type Value = Credential;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("string or map")
  }

  fn visit_str<E>(self, value: &str) -> Result<Credential, E>
  where
    E: serde::de::Error,
  {
    Ok(Credential {
      value: value.to_string(),
    })
  }

  fn visit_string<E>(self, value: String) -> Result<Credential, E>
  where
    E: serde::de::Error,
  {
    Ok(Credential { value })
  }
}

#[cfg_attr(feature = "cargo-clippy", allow(deprecated))]
impl<'de> Deserialize<'de> for Credential {
  fn deserialize<D>(deserializer: D) -> Result<Credential, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_any(CredentialVisitor)
  }
}

pub fn request_value(credential_key: &str, store_code: &str) -> Result<Value, String> {
  let backend_endpoint = get_store_hostname(store_code);
  let backend_username = get_store_username(store_code);
  let backend_password = get_store_password(store_code);

  let session_url = format!("{}/sessions", backend_endpoint);
  let credential_url = format!("{}/credentials/{}", backend_endpoint, credential_key);

  let client = Client::builder().build().map_err(|e| format!("{:?}", e))?;

  let session_body = SessionBody {
    session: Session {
      email: backend_username,
      password: backend_password,
    },
  };

  let response: SessionResponseBody = client
    .post(&session_url)
    .json(&session_body)
    .send()
    .map_err(|e| e.to_string())?
    .json()
    .map_err(|e| e.to_string())?;

  let mut headers = HeaderMap::new();

  headers.insert(
    AUTHORIZATION,
    HeaderValue::from_str(&response.access_token).map_err(|e| format!("{:?}", e))?,
  );

  let client = Client::builder()
    .default_headers(headers)
    .build()
    .map_err(|e| e.to_string())?;

  let response: ValueResponseBody = client
    .get(&credential_url)
    .send()
    .map_err(|e| e.to_string())?
    .json()
    .map_err(|e| e.to_string())?;

  Ok(response.data.value)
}
