use crate::parameter::media_segment::MediaSegment;
use crate::Credential;
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
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError>
  where
    Self: Sized;
  fn get_type_as_string() -> String;
}

impl ParameterValue for String {
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError> {
    if let serde_json::Value::String(content) = content {
      Ok(content.clone())
    } else {
      Err(ParameterValueError::new(&format!(
        "Could not find {} content to parse.",
        Self::get_type_as_string()
      )))
    }
  }

  fn get_type_as_string() -> String {
    "string".to_string()
  }
}

impl ParameterValue for i64 {
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError> {
    if let serde_json::Value::Number(content) = content {
      Ok(content.as_i64().unwrap())
    } else {
      Err(ParameterValueError::new(&format!(
        "Could not find {} content to parse.",
        Self::get_type_as_string()
      )))
    }
  }

  fn get_type_as_string() -> String {
    "integer".to_string()
  }
}

impl ParameterValue for f64 {
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError> {
    if let serde_json::Value::Number(content) = content {
      Ok(content.as_f64().unwrap())
    } else {
      Err(ParameterValueError::new(&format!(
        "Could not find {} content to parse.",
        Self::get_type_as_string()
      )))
    }
  }

  fn get_type_as_string() -> String {
    "float".to_string()
  }
}

impl ParameterValue for bool {
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError> {
    if let serde_json::Value::Bool(content) = content {
      Ok(*content)
    } else {
      Err(ParameterValueError::new(&format!(
        "Could not find {} content to parse.",
        Self::get_type_as_string()
      )))
    }
  }

  fn get_type_as_string() -> String {
    "boolean".to_string()
  }
}

impl ParameterValue for Vec<String> {
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError> {
    if let serde_json::Value::Array(array) = content {
      let mut ret = Vec::<String>::new();
      for item in array.iter() {
        if let serde_json::Value::String(value) = item {
          ret.push(value.clone());
        }
      }
      Ok(ret)
    } else {
      Err(ParameterValueError::new(&format!(
        "Could not find {} content to parse.",
        Self::get_type_as_string()
      )))
    }
  }

  fn get_type_as_string() -> String {
    "array_of_strings".to_string()
  }
}

impl ParameterValue for Credential {
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError> {
    if let serde_json::Value::String(content) = content {
      Ok(Credential {
        key: content.clone(),
      })
    } else {
      Err(ParameterValueError::new(&format!(
        "Could not find {} content to parse.",
        Self::get_type_as_string()
      )))
    }
  }

  fn get_type_as_string() -> String {
    "credential".to_string()
  }
}

impl ParameterValue for Requirement {
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError> {
    if let serde_json::Value::Object(content) = content {
      if let Some(paths_value) = content.get("paths") {
        let paths = Vec::<String>::parse_value(paths_value).map_err(|_e| {
          ParameterValueError::new(&format!(
            "Could not parse paths into {}.",
            Self::get_type_as_string()
          ))
        })?;
        return Ok(Requirement { paths: Some(paths) });
      }
    }
    Err(ParameterValueError::new(&format!(
      "Could not find {} content to parse.",
      Self::get_type_as_string()
    )))
  }

  fn get_type_as_string() -> String {
    "requirements".to_string()
  }
}

impl ParameterValue for Vec<MediaSegment> {
  fn parse_value(content: &serde_json::Value) -> Result<Self, ParameterValueError> {
    if let serde_json::Value::Array(array) = content {
      let mut ret = Vec::<MediaSegment>::new();
      for item in array.iter() {
        let media_segment =
          serde_json::from_value::<MediaSegment>(item.clone()).map_err(|error| {
            ParameterValueError::new(&format!(
              "Could not deserialize {} value: {:?}",
              Self::get_type_as_string(),
              error
            ))
          })?;
        ret.push(media_segment);
      }
      Ok(ret)
    } else {
      Err(ParameterValueError::new(&format!(
        "Could not find {} content to parse.",
        Self::get_type_as_string()
      )))
    }
  }

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
