//! Module to manage Job

use crate::{parameter::container::ParametersContainer, MessageError, Parameter, Requirement};
use serde_json::{Map, Value};
use std::path::Path;

mod job_progression;
mod job_result;
mod job_status;

use crate::parameter::store::request_value;
use crate::Result;
pub use job_progression::JobProgression;
pub use job_result::JobResult;
pub use job_status::JobStatus;
use serde::de::DeserializeOwned;
use serde::Deserialize;

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
  pub value: Value,
  inserted_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ValueResponseBody {
  pub data: DataResponseBody,
}

impl Job {
  pub fn new(message: &str) -> Result<Self> {
    let parsed: std::result::Result<Job, _> = serde_json::from_str(message);
    parsed
      .map_err(|e| MessageError::RuntimeError(format!("unable to parse input message: {:?}", e)))
  }

  pub fn get_parameters<P: Sized + DeserializeOwned>(&self) -> Result<P> {
    let mut parameters = Map::<String, Value>::new();
    for parameter in &self.parameters {
      if let Some(value) = parameter
        .value
        .clone()
        .or_else(|| parameter.default.clone())
      {
        let value = if let Some(store_code) = &parameter.store {
          debug!(
            "Retrieve credential value {} from store {}",
            value.to_string(),
            store_code
          );

          if let Value::String(credential_key) = value {
            request_value(&credential_key, &store_code)
              .map_err(|e| MessageError::ParameterValueError(format!("{:?}", e)))
          } else {
            Err(MessageError::ParameterValueError(format!(
              "Cannot handle credential type for {:?}",
              value
            )))
          }?
        } else {
          value
        };
        parameters.insert(parameter.id.clone(), value);
      }
    }
    let parameters = serde_json::Value::Object(parameters);

    serde_json::from_value(parameters.clone()).map_err(|error| {
      MessageError::ParameterValueError(format!(
        "Cannot get parameters from {:?}: {:?}",
        parameters, error
      ))
    })
  }

  pub fn check_requirements(&self) -> Result<()> {
    if let Ok(requirements) = self.get_parameter::<Requirement>("requirements") {
      if let Some(paths) = requirements.paths {
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
    Ok(())
  }
}

impl ParametersContainer for Job {
  fn get_parameters(&self) -> &Vec<Parameter> {
    &self.parameters
  }
}
