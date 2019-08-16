use std::path::Path;
use std::thread;

use reqwest::Error;

use crate::config;
use crate::MessageError;
use std::collections::HashMap;

pub trait ParametersContainer {
  fn get_parameters(&self) -> &Vec<Parameter>;
  fn get_boolean_parameter(&self, key: &str) -> Option<bool> {
    for param in self.get_parameters() {
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

  fn get_credential_parameter(&self, key: &str) -> Option<Credential> {
    for param in self.get_parameters() {
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

  fn get_integer_parameter(&self, key: &str) -> Option<i64> {
    for param in self.get_parameters() {
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

  fn get_string_parameter(&self, key: &str) -> Option<String> {
    for param in self.get_parameters() {
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

  fn get_array_of_strings_parameter(&self, key: &str) -> Option<Vec<String>> {
    for param in self.get_parameters() {
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

  fn get_parameters_as_map(&self) -> HashMap<String, Option<String>> {
    let mut map = HashMap::new();
    for param in self.get_parameters() {
      map.insert(param.get_id(), param.get_value_as_string());
    }
    map
  }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Requirement {
  paths: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

impl Parameter {
  pub fn get_id(&self) -> String {
    match self {
      Parameter::ArrayOfStringsParam { id, ..} |
      Parameter::BooleanParam { id, ..} |
      Parameter::CredentialParam { id, ..} |
      Parameter::IntegerParam { id, ..} |
      Parameter::RequirementParam { id, ..} |
      Parameter::StringParam { id, ..} => id.clone()
    }
  }

  pub fn get_value_as_string(&self) -> Option<String> {
    match self {
      Parameter::ArrayOfStringsParam { value, default, .. } => {
        if let Some(value) = value {
          Some(format!("{:?}", value))
        } else if let Some(default) = default {
          Some(format!("{:?}", default))
        } else {
          None
        }
      },
      Parameter::BooleanParam { value, default, .. } => {
        if let Some(value) = value {
          Some(format!("{}", value))
        } else if let Some(default) = default {
          Some(format!("{}", default))
        } else {
          None
        }
      },
      Parameter::CredentialParam { value, default, .. } => {
        if let Some(value) = value {
          Some(format!("{}", value))
        } else if let Some(default) = default {
          Some(format!("{}", default))
        } else {
          None
        }
      },
      Parameter::IntegerParam {  value, default, .. } => {
        if let Some(value) = value {
          Some(format!("{}", value))
        } else if let Some(default) = default {
          Some(format!("{}", default))
        } else {
          None
        }
      },
      Parameter::RequirementParam {  value, default, .. } => {
        if let Some(value) = value {
          Some(format!("{:?}", value))
        } else if let Some(default) = default {
          Some(format!("{:?}", default))
        } else {
          None
        }
      },
      Parameter::StringParam { value, default, .. } => {
        if let Some(value) = value {
          Some(format!("{}", value))
        } else if let Some(default) = default {
          Some(format!("{}", default))
        } else {
          None
        }
      }
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    let backend_endpoint = config::get_backend_hostname();
    let backend_username = config::get_backend_username();
    let backend_password = config::get_backend_password();

    let session_url = format!("{}/sessions", backend_endpoint);
    let credential_url = format!("{}/credentials/{}", backend_endpoint, self.key);

    let cloned_job = job.clone();
    let thread_job = job.clone();

    let request_thread = thread::spawn(move || {
      let client = reqwest::Client::builder().build().unwrap();

      let session_body = SessionBody {
        session: Session {
          email: backend_username,
          password: backend_password,
        },
      };

      let request = client.post(&session_url).json(&session_body).send();

      let mut response = check_error(request, &thread_job)?;

      let r: SessionResponseBody = response.json().map_err(|e| {
        let job_result = JobResult::from(&thread_job)
          .with_status(JobStatus::Error)
          .with_error(e);
        MessageError::ProcessingError(job_result)
      })?;
      let token = r.access_token;

      let request = client
        .get(&credential_url)
        // .bearer_auth(token)
        .header("Authorization", token)
        .send();

      let response = check_error(request, &thread_job)?;
      let resp_value = parse_json(response, &thread_job)?;

      Ok(resp_value.data.value)
    });

    request_thread.join().map_err(|e| {
      let job_result = JobResult::from(cloned_job)
        .with_status(JobStatus::Error)
        .with_message(format!("{:?}", e));
      MessageError::ProcessingError(job_result)
    })?
  }
}

impl ParametersContainer for Job {
  fn get_parameters(&self) -> &Vec<Parameter> {
    &self.parameters
  }
}

impl Job {
  pub fn new(message: &str) -> Result<Self, MessageError> {
    let parsed: Result<Job, _> = serde_json::from_str(message);
    parsed
      .map_err(|e| MessageError::RuntimeError(format!("unable to parse input message: {:?}", e)))
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
  #[serde(rename = "unknown")]
  Unknown,
  #[serde(rename = "completed")]
  Completed,
  #[serde(rename = "error")]
  Error,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct JobResult {
  pub job_id: u64,
  pub status: JobStatus,
  pub parameters: Vec<Parameter>,
}

impl From<Job> for JobResult {
  fn from(job: Job) -> JobResult {
    JobResult::new(job.job_id, JobStatus::Unknown, vec![])
  }
}

impl From<&Job> for JobResult {
  fn from(job: &Job) -> JobResult {
    JobResult::new(job.job_id, JobStatus::Unknown, vec![])
  }
}

impl ParametersContainer for JobResult {
  fn get_parameters(&self) -> &Vec<Parameter> {
    &self.parameters
  }
}

impl JobResult {
  pub fn new(job_id: u64, status: JobStatus, parameters: Vec<Parameter>) -> JobResult {
    JobResult {
      job_id,
      status,
      parameters,
    }
  }

  pub fn with_status(mut self, status: JobStatus) -> Self {
    self.status = status;
    self
  }

  pub fn with_error(mut self, error: Error) -> Self {
    self.parameters.push(Parameter::StringParam {
      id: "message".to_string(),
      default: None,
      value: Some(error.to_string()),
    });
    self
  }

  pub fn with_message(mut self, message: String) -> Self {
    self.parameters.push(Parameter::StringParam {
      id: "message".to_string(),
      default: None,
      value: Some(message),
    });
    self
  }
}

fn check_error<T>(item: Result<T, Error>, job: &Job) -> Result<T, MessageError> {
  item.map_err(|e| {
    let job_result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_error(e);
    MessageError::ProcessingError(job_result)
  })
}

fn parse_json(mut body: reqwest::Response, job: &Job) -> Result<ValueResponseBody, MessageError> {
  body.json().map_err(|e| {
    let job_result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_error(e);
    MessageError::ProcessingError(job_result)
  })
}
