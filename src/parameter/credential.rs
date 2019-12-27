use crate::{
  config::*,
  job::{Job, JobResult, JobStatus, Session, SessionBody, SessionResponseBody, ValueResponseBody},
  MessageError,
};
use reqwest::Error;
use std::thread;

#[derive(Debug, PartialEq)]
pub struct Credential {
  pub key: String,
}

impl Credential {
  pub fn request_value(&self, job: &Job) -> Result<String, MessageError> {
    let backend_endpoint = get_backend_hostname();
    let backend_username = get_backend_username();
    let backend_password = get_backend_password();

    let session_url = format!("{}/sessions", backend_endpoint);
    let credential_url = format!("{}/credentials/{}", backend_endpoint, self.key);

    let cloned_job = job.clone();
    let thread_job = job.clone();

    let request_thread = thread::spawn(move || {
      let client = reqwest::Client::builder().build().unwrap();

      let session_body = SessionBody {
        session: Session {
          email: backend_username,
          password: backend_password,
        },
      };

      let request = client.post(&session_url).json(&session_body).send();

      let mut response = check_error(request, &thread_job)?;

      let r: SessionResponseBody = response.json().map_err(|e| {
        let job_result = JobResult::from(&thread_job)
          .with_status(JobStatus::Error)
          .with_error(e);
        MessageError::ProcessingError(job_result)
      })?;
      let token = r.access_token;

      let request = client
        .get(&credential_url)
        // .bearer_auth(token)
        .header("Authorization", token)
        .send();

      let response = check_error(request, &thread_job)?;
      let resp_value = parse_json(response, &thread_job)?;

      Ok(resp_value.data.value)
    });

    request_thread.join().map_err(|e| {
      let job_result = JobResult::from(cloned_job)
        .with_status(JobStatus::Error)
        .with_message(&format!("{:?}", e));
      MessageError::ProcessingError(job_result)
    })?
  }
}

fn check_error<T>(item: Result<T, Error>, job: &Job) -> Result<T, MessageError> {
  item.map_err(|e| {
    let job_result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_error(e);
    MessageError::ProcessingError(job_result)
  })
}

fn parse_json(mut body: reqwest::Response, job: &Job) -> Result<ValueResponseBody, MessageError> {
  body.json().map_err(|e| {
    let job_result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_error(e);
    MessageError::ProcessingError(job_result)
  })
}
