use crate::job::{JobResult, JobStatus};

/// Internal error status to manage process errors
#[derive(Debug, PartialEq)]
pub enum MessageError {
  RuntimeError(String),
  ProcessingError(JobResult),
  RequirementsError(String),
  NotImplemented(),
}

impl MessageError {
  pub fn from(error: std::io::Error, job_result: JobResult) -> Self {
    let result = job_result
      .with_status(JobStatus::Error)
      .with_message(&format!("IO Error: {}", error.to_string()));

    MessageError::ProcessingError(result)
  }
}
