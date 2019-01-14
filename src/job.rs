use std::path::Path;
use MessageError;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Requirement {
  paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Parameter {
  #[serde(rename = "boolean")]
  BooleanParam {
    id: String,
    default: Option<bool>,
    value: Option<bool>,
  },
  #[serde(rename = "credential")]
  CredentialParam {
    id: String,
    default: Option<String>,
    value: Option<String>,
  },
  #[serde(rename = "integer")]
  IntegerParam {
    id: String,
    default: Option<i64>,
    value: Option<i64>,
  },
  #[serde(rename = "paths")]
  PathsParam {
    id: String,
    default: Option<Vec<String>>,
    value: Option<Vec<String>>,
  },
  #[serde(rename = "requirements")]
  RequirementParam {
    id: String,
    default: Option<Requirement>,
    value: Option<Requirement>,
  },
  #[serde(rename = "string")]
  StringParam {
    id: String,
    default: Option<String>,
    value: Option<String>,
  },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
  pub job_id: u64,
  pub parameters: Vec<Parameter>,
}

impl Job {
  pub fn new(message: &str) -> Result<Self, MessageError> {
    let parsed: Result<Job, _> = serde_json::from_str(message);
    parsed.map_err(|e| MessageError::RuntimeError(format!("unable to parse input message: {:?}", e)))
  }

  pub fn get_boolean_parameter(&self, key: &str) -> Option<bool> {
    for param in self.parameters.iter() {
      if let Parameter::BooleanParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.clone());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  pub fn get_credential_parameter(&self, key: &str) -> Option<String> {
    for param in self.parameters.iter() {
      if let Parameter::CredentialParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.clone());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  pub fn get_integer_parameter(&self, key: &str) -> Option<i64> {
    for param in self.parameters.iter() {
      if let Parameter::IntegerParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.clone());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  pub fn get_string_parameter(&self, key: &str) -> Option<String> {
    for param in self.parameters.iter() {
      if let Parameter::StringParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.clone());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  pub fn get_paths_parameter(&self, key: &str) -> Option<Vec<String>> {
    for param in self.parameters.iter() {
      if let Parameter::PathsParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.clone());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  pub fn check_requirements(&self) -> Result<(), MessageError> {
    for param in self.parameters.iter() {
      if let Parameter::RequirementParam { id, value, .. } = param {
        if id == "requirements" {
          if let Some(Requirement { paths: Some(paths) }) = value {
            for path in paths.iter() {
              let p = Path::new(path);
              if !p.exists() {
                return Err(MessageError::RequirementsError(format!(
                  "Warning: Required file does not exists: {:?}",
                  p
                )));
              }
            }
          }
        }
      }
    }
    Ok(())
  }
}
