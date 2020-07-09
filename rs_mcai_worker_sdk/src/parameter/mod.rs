use crate::parameter::media_segment::MediaSegment;
use crate::MessageError;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

pub mod container;
pub mod credential;
pub mod media_segment;

pub trait ParameterValue {
  fn parse_value(content: Value, store: &Option<String>) -> Result<Self, MessageError>
  where
    Self: Sized + DeserializeOwned,
  {
    let content = if let Some(store_code) = store {
      debug!(
        "Retrieve credential value {} from store {}",
        content.to_string(),
        store_code
      );

      if let Value::String(credential_key) = content {
        Self::from_store(&credential_key, &store_code)
      } else {
        Err(MessageError::ParameterValueError(format!(
          "Cannot handle credential type for {:?}",
          content
        )))
      }?
    } else {
      content
    };

    Self::from_value(content)
  }

  fn from_store(key: &str, store_code: &str) -> Result<Value, MessageError> {
    credential::request_value(&key, &store_code)
      .map_err(|e| MessageError::ParameterValueError(format!("{:?}", e)))
  }

  fn from_value(content: Value) -> Result<Self, MessageError>
  where
    Self: Sized + DeserializeOwned,
  {
    serde_json::value::from_value(content)
      .map_err(|e| MessageError::ParameterValueError(format!("{:?}", e)))
  }

  fn get_type_as_string() -> String;
}

impl ParameterValue for String {
  fn get_type_as_string() -> String {
    "string".to_string()
  }
}

impl ParameterValue for i64 {
  fn from_value(value: Value) -> Result<i64, MessageError> {
    match value {
      Value::String(value) => value
        .parse()
        .map_err(|e| MessageError::ParameterValueError(format!("{:?}", e))),
      Value::Number(value) => value.as_i64().ok_or_else(|| {
        MessageError::ParameterValueError(format!(
          "Cannot convert value type '{:?}' to type {}",
          value,
          std::any::type_name::<Self>()
        ))
      }),
      _ => Err(MessageError::ParameterValueError(format!(
        "Cannot convert value type '{:?}' to type {}",
        value,
        std::any::type_name::<Self>()
      ))),
    }
  }

  fn get_type_as_string() -> String {
    "integer".to_string()
  }
}

impl ParameterValue for f64 {
  fn from_value(value: Value) -> Result<f64, MessageError> {
    match value {
      Value::String(value) => value
        .parse()
        .map_err(|e| MessageError::ParameterValueError(format!("{:?}", e))),
      Value::Number(value) => value.as_f64().ok_or_else(|| {
        MessageError::ParameterValueError(format!(
          "Cannot convert value type '{:?}' to type {}",
          value,
          std::any::type_name::<Self>()
        ))
      }),
      _ => Err(MessageError::ParameterValueError(format!(
        "Cannot convert value type '{:?}' to type {}",
        value,
        std::any::type_name::<Self>()
      ))),
    }
  }

  fn get_type_as_string() -> String {
    "float".to_string()
  }
}

impl ParameterValue for bool {
  fn from_value(value: Value) -> Result<bool, MessageError> {
    match value {
      Value::String(value) => value
        .parse()
        .map_err(|e| MessageError::ParameterValueError(format!("{:?}", e))),
      Value::Number(value) => Ok(value.as_i64().map_or_else(|| false, |v| v > 0)),
      Value::Bool(value) => Ok(value),
      _ => Err(MessageError::ParameterValueError(format!(
        "Cannot convert value type '{:?}' to type {}",
        value,
        std::any::type_name::<Self>()
      ))),
    }
  }

  fn get_type_as_string() -> String {
    "boolean".to_string()
  }
}

impl ParameterValue for Vec<String> {
  fn get_type_as_string() -> String {
    "array_of_strings".to_string()
  }
}

#[cfg_attr(feature = "cargo-clippy", allow(deprecated))]
impl ParameterValue for credential::Credential {
  fn parse_value(content: Value, store: &Option<String>) -> Result<Self, MessageError>
  where
    Self: Sized + DeserializeOwned,
  {
    let store_code = store.clone().unwrap_or_else(|| "BACKEND".to_string());

    debug!(
      "Retrieve credential value {} from store {}",
      content.to_string(),
      store_code
    );

    if let Value::String(credential_key) = &content {
      let value = Self::from_store(&credential_key, &store_code)?;
      Self::from_value(value)
    } else {
      Err(MessageError::ParameterValueError(format!(
        "Cannot handle credential type for {:?}",
        content
      )))
    }
  }

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
  pub store: Option<String>,
  pub value: Option<Value>,
  pub default: Option<Value>,
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
      Value::Null => format!("{:?}", current_value),
      Value::Object(_content) => serde_json::to_string(current_value).unwrap(),
      Value::Array(_content) => serde_json::to_string(current_value).unwrap(),
      Value::Bool(content) => format!("{}", content),
      Value::Number(content) => format!("{}", content),
      Value::String(content) => content.to_string(),
    }
  }
}
