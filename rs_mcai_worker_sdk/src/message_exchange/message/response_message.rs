use super::Feedback;
use crate::{
  job::{JobResult, JobStatus},
  worker::WorkerConfiguration,
  MessageError,
};

/// Message from the Worker to the Message Exchange
#[derive(Clone, Debug, PartialEq)]
pub enum ResponseMessage {
  Completed(JobResult),
  Feedback(Feedback),
  JobStopped(JobResult),
  Error(MessageError),
  StatusError(MessageError),
  WorkerCreated(Box<WorkerConfiguration>),
  WorkerInitialized(JobResult),
  WorkerStarted(JobResult),
}

impl Into<JobStatus> for ResponseMessage {
  fn into(self) -> JobStatus {
    match self {
      ResponseMessage::Completed(_) => JobStatus::Completed,
      ResponseMessage::Error(_) => JobStatus::Error,
      _ => JobStatus::Unknown,
    }
  }
}
