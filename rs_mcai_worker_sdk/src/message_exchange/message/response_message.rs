use super::Feedback;
use crate::{job::JobResult, worker::WorkerConfiguration, MessageError};

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
