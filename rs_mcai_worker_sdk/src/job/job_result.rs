use super::job_status::JobStatus;
use crate::job::Job;
use crate::parameter::container::ParametersContainer;
use crate::parameter::Parameter;
use crate::parameter::ParameterValue;
use reqwest::Error;
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobResult {
  destination_paths: Vec<String>,
  execution_duration: f64,
  job_id: u64,
  parameters: Vec<Parameter>,
  #[serde(skip_serializing, skip_deserializing, default = "default_instant")]
  start_instant: Instant,
  status: JobStatus,
}

fn default_instant() -> Instant {
  Instant::now()
}

impl JobResult {
  pub fn new(job_id: u64) -> JobResult {
    JobResult {
      destination_paths: vec![],
      execution_duration: 0.0,
      job_id,
      parameters: vec![],
      start_instant: Instant::now(),
      status: JobStatus::default(),
    }
  }

  pub fn with_status(mut self, status: JobStatus) -> Self {
    self.update_execution_duration();
    self.status = status;
    self
  }

  pub fn with_error(mut self, error: Error) -> Self {
    self.update_execution_duration();
    self.parameters.push(Parameter {
      id: "message".to_string(),
      kind: String::get_type_as_string(),
      store: None,
      default: None,
      value: serde_json::to_value(error.to_string()).ok(),
    });
    self
  }

  pub fn with_message(mut self, message: &str) -> Self {
    self.parameters.push(Parameter {
      id: "message".to_string(),
      kind: String::get_type_as_string(),
      store: None,
      default: None,
      value: serde_json::to_value(message.to_string()).ok(),
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

  pub fn with_json<T>(mut self, id: &str, serializable: &T) -> Result<Self, String>
  where
    T: Serialize + Sized,
  {
    let json_string = serde_json::to_string(serializable)
      .map_err(|error| format!("Unable to serialize object: {:?}", error))?;
    self.parameters.push(Parameter {
      id: id.to_string(),
      kind: String::get_type_as_string(),
      store: None,
      default: None,
      value: serde_json::to_value(json_string).ok(),
    });
    Ok(self)
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

  pub fn get_execution_duration(&self) -> f64 {
    self.execution_duration
  }

  pub fn get_parameters(&self) -> &Vec<Parameter> {
    &self.parameters
  }

  pub fn get_destination_paths(&self) -> &Vec<String> {
    &self.destination_paths
  }

  pub fn update_execution_duration(&mut self) {
    self.execution_duration = self.start_instant.elapsed().as_secs_f64();
  }
}

impl From<Job> for JobResult {
  fn from(job: Job) -> JobResult {
    JobResult::new(job.job_id)
  }
}

impl From<&Job> for JobResult {
  fn from(job: &Job) -> JobResult {
    JobResult::new(job.job_id)
  }
}

impl ParametersContainer for JobResult {
  fn get_parameters(&self) -> &Vec<Parameter> {
    &self.parameters
  }
}

impl PartialEq for JobResult {
  fn eq(&self, other: &Self) -> bool {
    self.job_id == other.job_id
      && self.status == other.status
      && self.parameters == other.parameters
      && self.destination_paths == other.destination_paths
  }
}
