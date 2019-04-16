use config;
use std::path::Path;
use MessageError;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Requirement {
  paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Parameter {
  #[serde(rename = "array_of_strings")]
  ArrayOfStringsParam {
    id: String,
    default: Option<Vec<String>>,
    value: Option<Vec<String>>,
  },
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

#[derive(Debug)]
pub struct Credential {
  pub key: String,
}

#[derive(Debug, Serialize)]
struct Session {
  email: String,
  password: String,
}

#[derive(Debug, Serialize)]
struct SessionBody {
  session: Session,
}

#[derive(Debug, Deserialize)]
struct SessionResponseBody {
  access_token: String,
}

#[derive(Debug, Deserialize)]
struct DataResponseBody {
  id: u32,
  key: String,
  value: String,
  inserted_at: String,
}

#[derive(Debug, Deserialize)]
struct ValueResponseBody {
  data: DataResponseBody,
}

impl Credential {
  pub fn request_value(&self, job: &Job) -> Result<String, MessageError> {
    let client = reqwest::Client::builder().build().unwrap();
    let backend_endpoint = config::get_backend_hostname();
    let backend_username = config::get_backend_username();
    let backend_password = config::get_backend_password();

    let session_body = SessionBody {
      session: Session {
        email: backend_username,
        password: backend_password,
      },
    };

    let mut response = client
      .post(&(backend_endpoint.clone() + "/sessions"))
      .json(&session_body)
      .send()
      .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;

    let r: SessionResponseBody = response
      .json()
      .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;
    let token = r.access_token;

    let mut response = client
      .get(&(backend_endpoint + "/credentials/" + &self.key))
      // .bearer_auth(token)
      .header("Authorization", token)
      .send()
      .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;

    let resp_value: ValueResponseBody = response
      .json()
      .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;
    Ok(resp_value.data.value)
  }
}

impl Job {
  pub fn new(message: &str) -> Result<Self, MessageError> {
    let parsed: Result<Job, _> = serde_json::from_str(message);
    parsed
      .map_err(|e| MessageError::RuntimeError(format!("unable to parse input message: {:?}", e)))
  }

  pub fn get_boolean_parameter(&self, key: &str) -> Option<bool> {
    for param in self.parameters.iter() {
      if let Parameter::BooleanParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(*v);
          } else {
            return *default;
          }
        }
      }
    }
    None
  }

  pub fn get_credential_parameter(&self, key: &str) -> Option<Credential> {
    for param in self.parameters.iter() {
      if let Parameter::CredentialParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(Credential { key: v.to_string() });
          } else {
            return default.clone().map(|key| Credential { key });
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
            return Some(*v);
          } else {
            return *default;
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
            return Some(v.to_string());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  pub fn get_array_of_strings_parameter(&self, key: &str) -> Option<Vec<String>> {
    for param in self.parameters.iter() {
      if let Parameter::ArrayOfStringsParam { id, default, value } = param {
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
