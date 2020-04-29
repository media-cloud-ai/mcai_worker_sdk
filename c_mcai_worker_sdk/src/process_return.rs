use amqp_worker::job::*;
use amqp_worker::MessageError;

#[derive(Debug)]
pub struct ProcessReturn {
  code: i32,
  message: String,
  output_paths: Vec<String>,
}

impl ProcessReturn {
  pub fn new(code: i32, message: &str) -> Self {
    ProcessReturn {
      code,
      message: message.to_string(),
      output_paths: vec![],
    }
  }

  pub fn new_error(message: &str) -> Self {
    ProcessReturn::new(ProcessReturn::get_error_code(), message)
  }

  pub fn with_output_paths(mut self, output_paths: Vec<String>) -> Self {
    self.output_paths = output_paths;
    self
  }

  pub fn get_error_code() -> i32 {
    1
  }

  #[allow(dead_code)]
  pub fn get_code(&self) -> i32 {
    self.code
  }

  #[allow(dead_code)]
  pub fn get_message(&self) -> &String {
    &self.message
  }

  #[allow(dead_code)]
  pub fn get_output_paths(&self) -> &Vec<String> {
    &self.output_paths
  }

  pub fn as_result(&self, job_result: JobResult) -> Result<JobResult, MessageError> {
    if self.code == 0 {
      let mut output_paths = self.output_paths.clone();

      let job_result = job_result
        .with_status(JobStatus::Completed)
        .with_destination_paths(&mut output_paths)
        .with_message(&self.message);

      Ok(job_result)
    } else {
      let result = job_result
        .with_status(JobStatus::Error)
        .with_message(&format!("{} (code: {:?})", self.message, self.code));

      Err(MessageError::ProcessingError(result))
    }
  }
}

#[test]
pub fn process_return_new() {
  let process_return = ProcessReturn::new(123, "this is a message");
  assert_eq!(123, process_return.get_code());
  assert_eq!(
    &"this is a message".to_string(),
    process_return.get_message()
  );
  assert_eq!(0, process_return.get_output_paths().len());
}

#[test]
pub fn process_return_new_with_output_paths() {
  let output_path = "/path/to/output";
  let mut output_paths = vec![];
  output_paths.push(output_path.to_string());
  let process_return = ProcessReturn::new(123, "this is a message").with_output_paths(output_paths);
  assert_eq!(123, process_return.get_code());
  assert_eq!(
    &"this is a message".to_string(),
    process_return.get_message()
  );
  assert_eq!(1, process_return.get_output_paths().len());
  assert_eq!(
    output_path,
    process_return.get_output_paths().get(0).unwrap()
  );
}

#[test]
pub fn process_return_new_error() {
  let process_return = ProcessReturn::new_error("this is an error message");
  assert_eq!(ProcessReturn::get_error_code(), process_return.get_code());
  assert_eq!(
    &"this is an error message".to_string(),
    process_return.get_message()
  );
  assert_eq!(0, process_return.get_output_paths().len());
}
