extern crate assert_matches;
extern crate mcai_worker_sdk;

use mcai_worker_sdk::job::{JobResult, JobStatus};
use mcai_worker_sdk::MessageError;

#[test]
pub fn test_message_error_from() {
  let error = std::io::Error::from(std::io::ErrorKind::NotFound);
  let job_result = JobResult::new(123);

  let message_error = MessageError::from(error, job_result.clone());
  let expected = MessageError::ProcessingError(
    job_result
      .with_status(JobStatus::Error)
      .with_message("IO Error: entity not found"),
  );
  assert_eq!(expected, message_error);
}
