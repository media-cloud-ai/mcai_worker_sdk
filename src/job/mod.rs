use crate::{parameter::container::ParametersContainer, MessageError, Parameter, Requirement};
use reqwest::Error;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
  pub job_id: u64,
  pub parameters: Vec<Parameter>,
}

#[derive(Debug, Serialize)]
pub struct Session {
  pub email: String,
  pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SessionBody {
  pub session: Session,
}

#[derive(Debug, Deserialize)]
pub struct SessionResponseBody {
  pub access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct DataResponseBody {
  id: u32,
  key: String,
  pub value: String,
  inserted_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ValueResponseBody {
  pub data: DataResponseBody,
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

impl ParametersContainer for Job {
  fn get_parameters(&self) -> &Vec<Parameter> {
    &self.parameters
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
  job_id: u64,
  status: JobStatus,
  parameters: Vec<Parameter>,
  destination_paths: Vec<String>,
}

impl From<Job> for JobResult {
  fn from(job: Job) -> JobResult {
    JobResult::new(job.job_id, JobStatus::Unknown)
  }
}

impl From<&Job> for JobResult {
  fn from(job: &Job) -> JobResult {
    JobResult::new(job.job_id, JobStatus::Unknown)
  }
}

impl ParametersContainer for JobResult {
  fn get_parameters(&self) -> &Vec<Parameter> {
    &self.parameters
  }
}

impl JobResult {
  pub fn new(job_id: u64, status: JobStatus) -> JobResult {
    JobResult {
      job_id,
      status,
      parameters: vec![],
      destination_paths: vec![]
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

  pub fn with_parameters(mut self, parameters: &mut Vec<Parameter>) -> Self {
    self.parameters.append(parameters);
    self
  }

  pub fn with_destination_paths(mut self, destination_paths: &mut Vec<String>) -> Self {
    self.destination_paths.append(destination_paths);
    self
  }

  pub fn get_job_id(&self) -> u64 {
    self.job_id
  }

  pub fn get_str_job_id(&self) -> String {
    self.job_id.to_string()
  }

  pub fn get_status(&self) -> &JobStatus {
    &self.status
  }

  pub fn get_parameters(&self) -> &Vec<Parameter> {
    &self.parameters
  }
}
