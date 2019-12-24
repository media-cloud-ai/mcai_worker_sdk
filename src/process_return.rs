
use amqp_worker::job::*;
use amqp_worker::MessageError;

#[derive(Debug)]
pub struct ProcessReturn {
  code: i32,
  message: String,
}

impl ProcessReturn {
  pub fn new(code: i32, message: &str) -> Self {
    ProcessReturn {
      code,
      message: message.to_string(),
    }
  }

  pub fn new_error(message: &str) -> Self {
    ProcessReturn::new(ProcessReturn::get_error_code(), message)
  }

  pub fn get_error_code() -> i32 {
    1
  }

  pub fn get_code(&self) -> i32 {
    self.code
  }

  pub fn get_message(&self) -> &String {
    &self.message
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
