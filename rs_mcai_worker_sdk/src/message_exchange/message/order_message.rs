use crate::{
  job::{Job, JobResult, JobStatus},
  MessageError, Result,
};
use std::convert::TryFrom;

/// Message to start actions on the worker itself
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrderMessage {
  Job(Job),
  InitProcess(Job),
  StartProcess(Job),
  StopProcess(Job),
  StopWorker,
  Status,
  StopConsumingJobs,
  ResumeConsumingJobs,
}

impl OrderMessage {
  pub fn matches_job_id(&self, job_id: Option<u64>) -> Result<()> {
    match self {
      OrderMessage::Job(job) | OrderMessage::InitProcess(job) => {
        if job_id.is_some() {
          build_error(
            job,
            "Cannot initialize this job, an another job is already in progress.",
          )?;
        }
      }
      OrderMessage::StartProcess(job) => {
        if job_id.is_none() {
          build_error(job, "Cannot start a not initialized job.")?;
        }
        if job_id != Some(job.job_id) {
          build_error(job, "The Job ID is not the same as the initialized job.")?;
        }
      }
      OrderMessage::StopProcess(job) => {
        if job_id.is_none() {
          build_error(job, "Cannot stop a non-running job.")?;
        }
        if job_id != Some(job.job_id) {
          build_error(job, "The Job ID is not the same as the current job.")?;
        }
      }
      _ => {}
    }
    Ok(())
  }
}

fn build_error(job: &Job, message: &str) -> Result<()> {
  Err(MessageError::ProcessingError(
    JobResult::new(job.job_id)
      .with_status(JobStatus::Error)
      .with_message(message),
  ))
}

impl TryFrom<&str> for OrderMessage {
  type Error = MessageError;

  fn try_from(message_data: &str) -> Result<OrderMessage> {
    match serde_json::from_str::<OrderMessage>(message_data) {
      Ok(order_message) => Ok(order_message),
      Err(error) => {
        if let Ok(job_order) = Job::new(message_data) {
          Ok(OrderMessage::Job(job_order))
        } else {
          Err(MessageError::RuntimeError(error.to_string()))
        }
      }
    }
  }
}
