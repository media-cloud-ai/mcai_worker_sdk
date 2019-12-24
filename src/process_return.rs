
use amqp_worker::job::*;
use amqp_worker::MessageError;

#[derive(Debug)]
pub struct ProcessReturn {
  pub code: i32,
  pub message: String,
}

impl ProcessReturn {
  pub fn new(code: i32, message: &str) -> Self {
    ProcessReturn {
      code,
      message: message.to_string(),
    }
  }

  pub fn new_error(message: &str) -> Self {
    ProcessReturn::new(1, message)
  }

  pub fn as_result(&self, job_id: u64) -> Result<JobResult, MessageError> {
    if self.code == 0 {
      let job_result =
        JobResult::new(job_id, JobStatus::Completed)
        .with_message(&self.message);

      Ok(job_result)
    } else {
      let result = JobResult::new(job_id, JobStatus::Error)
        .with_message(&format!("{} (code: {:?})", self.message, self.code));

      Err(MessageError::ProcessingError(result))
    }
  }
}
