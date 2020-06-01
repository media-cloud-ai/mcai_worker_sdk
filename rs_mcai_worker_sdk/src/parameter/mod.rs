use crate::parameter::media_segment::MediaSegment;
use crate::Credential;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;

pub mod container;
pub mod credential;
pub mod media_segment;

#[derive(Debug, PartialEq)]
pub struct ParameterValueError {
  description: String,
}

impl ParameterValueError {
  fn new(message: &str) -> ParameterValueError {
    ParameterValueError {
      description: message.to_string(),
    }
  }
}

impl Error for ParameterValueError {
  fn description(&self) -> &str {
    self.description.as_ref()
  }
}

impl std::fmt::Display for ParameterValueError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.write_str(&self.to_string())
  }
}

pub trait ParameterValue {
  fn parse_value(
    content: serde_json::Value,
  ) -> Result<Self, ParameterValueError>
  where
    Self: Sized + DeserializeOwned,
  {
    serde_json::value::from_value(content)
      .map_err(|e| ParameterValueError::new(&format!("{:?}", e)))
  }
  fn get_type_as_string() -> String;
}

impl ParameterValue for String {
  fn get_type_as_string() -> String {
    "string".to_string()
  }
}

impl ParameterValue for i64 {
  fn get_type_as_string() -> String {
    "integer".to_string()
  }
}

impl ParameterValue for f64 {
  fn get_type_as_string() -> String {
    "float".to_string()
  }
}

impl ParameterValue for bool {
  fn get_type_as_string() -> String {
    "boolean".to_string()
  }
}

impl ParameterValue for Vec<String> {
  fn get_type_as_string() -> String {
    "array_of_strings".to_string()
  }
}

impl ParameterValue for Credential {
  fn get_type_as_string() -> String {
    "credential".to_string()
  }
}

impl ParameterValue for Requirement {
  fn get_type_as_string() -> String {
    "requirements".to_string()
  }
}

impl ParameterValue for Vec<MediaSegment> {
  fn get_type_as_string() -> String {
    "array_of_media_segments".to_string()
  }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Requirement {
  pub paths: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Parameter {
  pub id: String,
  #[serde(rename = "type")]
  pub kind: String,
  pub value: Option<serde_json::Value>,
  pub default: Option<serde_json::Value>,
}

impl Parameter {
  pub fn get_id(&self) -> String {
    self.id.clone()
  }

  pub fn has_value_or_default(&self) -> bool {
    self.value.is_some() || self.default.is_some()
  }
}

impl ToString for Parameter {
  fn to_string(&self) -> String {
    let current_value = if let Some(value) = &self.value {
      value
    } else if let Some(default) = &self.default {
      default
    } else {
      return "".to_string();
    };

    match current_value {
      serde_json::Value::Null => format!("{:?}", current_value),
      serde_json::Value::Object(_content) => serde_json::to_string(current_value).unwrap(),
      serde_json::Value::Array(_content) => serde_json::to_string(current_value).unwrap(),
      serde_json::Value::Bool(content) => format!("{}", content),
      serde_json::Value::Number(content) => format!("{}", content),
      serde_json::Value::String(content) => content.to_string(),
    }
  }
}
