use std::path::Path;
use MessageError;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Requirement {
  paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Parameter {
  #[serde(rename = "string")]
  StringParam {
    id: String,
    default: Option<String>,
    value: Option<String>,
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
  pub job_id: u64,
  pub parameters: Vec<Parameter>,
}

impl Job {
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

  pub fn check_requirements(&self) -> Result<(), MessageError> {
    for param in self.parameters.iter() {
      if let Parameter::RequirementParam { id, value, default: _ } = param {
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
