//! Module to manage Job

use crate::{parameter::container::ParametersContainer, MessageError, Parameter, Requirement};
use std::path::Path;

mod job_progression;
mod job_result;
mod job_status;

pub use job_progression::JobProgression;
pub use job_result::JobResult;
pub use job_status::JobStatus;

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
