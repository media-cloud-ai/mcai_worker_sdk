use crate::{
  config::*,
  job::{Session, SessionBody, SessionResponseBody, ValueResponseBody},
};
use reqwest::{
  blocking::Client,
  header::{HeaderMap, HeaderValue, AUTHORIZATION},
};
use serde_json::Value;
use std::env::var;

pub fn request_value(credential_key: &str, store_code: &str) -> Result<Value, String> {
  match store_code.to_lowercase().as_str() {
    "env" | "environment" => var(credential_key)
      .map_err(|error| error.to_string())
      .map(|value| serde_json::from_str(&value).unwrap_or(Value::String(value))),
    _ => {
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
        .error_for_status()
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
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

      let value = match response.data.value.clone() {
        Value::String(string) => serde_json::from_str(&string).unwrap_or(response.data.value),
        _ => response.data.value,
      };

      Ok(value)
    }
  }
}
