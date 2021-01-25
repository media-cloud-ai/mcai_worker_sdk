use crate::job::JobProgression;
use crate::worker::{WorkerActivity, WorkerStatus};
use crate::{
  job::{Job, JobResult, JobStatus},
  message_exchange::{Feedback, OrderMessage, ResponseMessage},
  processor::{Process, ProcessStatus},
  worker::{SystemInformation, WorkerConfiguration},
  McaiChannel, MessageError, MessageEvent, Result,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

pub struct SimpleProcess {
  response_sender: McaiChannel,
  status: JobStatus,
  worker_configuration: WorkerConfiguration,
  current_job_id: Option<u64>,
}

impl<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send> Process<P, ME>
  for SimpleProcess
{
  fn new(
    _message_event: Arc<Mutex<ME>>,
    response_sender: McaiChannel,
    worker_configuration: WorkerConfiguration,
  ) -> Self {
    SimpleProcess {
      response_sender,
      status: JobStatus::Unknown,
      worker_configuration,
      current_job_id: None,
    }
  }

  fn handle(&mut self, message_event: Arc<Mutex<ME>>, order_message: OrderMessage) -> Result<()> {
    let response = self.handle_message(message_event, order_message)?;
    self.response_sender.lock().unwrap().send_response(response)
  }

  fn get_current_job_id(&self, _message_event: Arc<Mutex<ME>>) -> Option<u64> {
    self.current_job_id
  }
}

impl SimpleProcess {
  fn get_worker_status(&self) -> WorkerStatus {
    let activity = self.get_worker_activity();
    let system_info = SystemInformation::new(&self.worker_configuration.clone());
    WorkerStatus::new(activity, system_info)
  }

  fn get_worker_activity(&self) -> WorkerActivity {
    match self.status {
      JobStatus::Initialized | JobStatus::Running => WorkerActivity::Busy,
      JobStatus::Completed | JobStatus::Error | JobStatus::Unknown => WorkerActivity::Idle,
    }
  }

  fn handle_message<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    order_message: OrderMessage,
  ) -> Result<ResponseMessage> {
    match order_message {
      OrderMessage::InitProcess(job) => {
        self.status = JobStatus::Initialized;
        self.current_job_id = Some(job.job_id);

        Ok(ResponseMessage::WorkerInitialized(
          JobResult::new(job.job_id).with_status(JobStatus::Initialized),
        ))
      }
      OrderMessage::Job(job) => {
        self.status = JobStatus::Initialized;
        self.current_job_id = Some(job.job_id);

        self
          .response_sender
          .lock()
          .unwrap()
          .send_response(ResponseMessage::WorkerInitialized(
            JobResult::new(job.job_id).with_status(JobStatus::Initialized),
          ))?;

        self.start_job(message_event, &job)
      }
      OrderMessage::StartProcess(job) => self.start_job(message_event, &job),
      OrderMessage::StopProcess(job) => {
        self.status = JobStatus::Completed;
        self.current_job_id = None;

        // TODO return ResponseMessage::Completed with JobResult when on started on thread
        // ResponseMessage::Completed(JobResult::new(job.job_id).with_status(JobStatus::Initialized))
        Ok(ResponseMessage::Error(MessageError::ProcessingError(
          JobResult::new(job.job_id)
            .with_status(JobStatus::Error)
            .with_message("Cannot stop a non-running job."),
        )))
      }
      OrderMessage::Status | OrderMessage::StopWorker => {
        let current_job_result = self
          .current_job_id
          .map(|job_id| JobResult::new(job_id).with_status(self.status.clone()));

        Ok(ResponseMessage::Feedback(Feedback::Status(
          ProcessStatus::new(self.get_worker_status(), current_job_result),
        )))
      }
    }
  }

  fn start_job<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    job: &Job,
  ) -> Result<ResponseMessage> {
    log::info!("Process job: {:?}", job);
    self.status = JobStatus::Running;

    // start publishing progression
    self
      .response_sender
      .lock()
      .unwrap()
      .send_response(ResponseMessage::Feedback(Feedback::Progression(
        JobProgression::new(job.job_id, 0),
      )))?;

    let response = message_event
      .lock()
      .unwrap()
      .process(
        Some(self.response_sender.clone()),
        job.get_parameters().unwrap(),
        JobResult::from(job),
      )
      .map(ResponseMessage::Completed)
      .unwrap_or_else(ResponseMessage::Error);

    self.status = match response {
      ResponseMessage::Completed(_) => JobStatus::Completed,
      ResponseMessage::Error(_) => JobStatus::Error,
      _ => JobStatus::Unknown,
    };

    self.current_job_id = None;

    Ok(response)
  }
}

impl Drop for SimpleProcess {
  fn drop(&mut self) {
    log::info!("Simple process dropped with status: {:?}", self.status);
  }
}
