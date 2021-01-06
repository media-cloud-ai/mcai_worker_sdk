use crate::job::JobProgression;
use crate::worker::status::{WorkerActivity, WorkerStatus};
use crate::{
  job::{JobResult, JobStatus},
  message_exchange::{Feedback, OrderMessage, ResponseMessage},
  processor::{Process, ProcessStatus},
  worker::{system_information::SystemInformation, WorkerConfiguration},
  McaiChannel, MessageError, MessageEvent, Result,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

pub struct SimpleProcess {
  response_sender: McaiChannel,
  status: JobStatus,
  worker_configuration: WorkerConfiguration,
  last_job_id: Option<u64>,
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
      last_job_id: None,
    }
  }

  fn handle(&mut self, message_event: Arc<Mutex<ME>>, order_message: OrderMessage) -> Result<()> {
    let response = match order_message {
      OrderMessage::InitProcess(job) => {
        self.status = JobStatus::Initialized;
        ResponseMessage::Initialized(JobResult::new(job.job_id).with_status(JobStatus::Initialized))
      }
      OrderMessage::Job(job) | OrderMessage::StartProcess(job) => {
        info!("Process job: {:?}", job);
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

        match response {
          ResponseMessage::Completed(_) => {
            self.status = JobStatus::Completed;
          }
          ResponseMessage::Error(_) => {
            self.status = JobStatus::Error;
          }
          _ => {
            self.status = JobStatus::Unknown;
          }
        }

        response
      }
      OrderMessage::StopProcess(job) => {
        self.status = JobStatus::Completed;
        self.last_job_id = Some(job.job_id);

        // TODO return ResponseMessage::Completed with JobResult when on started on thread
        // ResponseMessage::Completed(JobResult::new(job.job_id).with_status(JobStatus::Initialized))
        ResponseMessage::Error(MessageError::ProcessingError(
          JobResult::new(job.job_id)
            .with_status(JobStatus::Error)
            .with_message("Cannot stop a non-running job."),
        ))
      }
      OrderMessage::Status | OrderMessage::StopWorker => {
        ResponseMessage::Feedback(Feedback::Status(ProcessStatus::new(
          self.get_worker_status(),
          self.get_last_job_result(),
        )))
      }
    };

    self.response_sender.lock().unwrap().send_response(response)
  }
}

impl SimpleProcess {
  fn get_last_job_result(&self) -> Option<JobResult> {
    self
      .last_job_id
      .map(|job_id| JobResult::new(job_id).with_status(self.status.clone()))
  }

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
}

impl Drop for SimpleProcess {
  fn drop(&mut self) {
    info!("Simple process dropped with status: {:?}", self.status);
  }
}
