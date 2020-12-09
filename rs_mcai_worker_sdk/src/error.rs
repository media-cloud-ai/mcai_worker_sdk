use crate::job::{JobResult, JobStatus};

#[derive(Debug, PartialEq)]
pub enum SdkError {
  Amqp(lapin::Error),
}

impl From<lapin::Error> for SdkError {
  fn from(error: lapin::Error) -> Self {
    SdkError::Amqp(error)
  }
}

/// Internal error status to manage process errors
#[derive(Clone, Debug, PartialEq)]
pub enum MessageError {
  Amqp(lapin::Error),
  RuntimeError(String),
  ParameterValueError(String),
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

impl From<lapin::Error> for MessageError {
  fn from(error: lapin::Error) -> Self {
    MessageError::Amqp(error)
  }
}

pub type Result<T> = std::result::Result<T, MessageError>;
pub type SdkResult<T> = std::result::Result<T, SdkError>;
