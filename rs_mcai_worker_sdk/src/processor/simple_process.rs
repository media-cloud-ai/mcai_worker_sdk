use crate::{
  job::{JobResult, JobStatus},
  message_exchange::{Feedback, OrderMessage, ResponseMessage},
  processor::{Process, ProcessStatus},
  worker::{system_information::SystemInformation, WorkerConfiguration},
  McaiChannel, MessageEvent, Result,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

pub struct SimpleProcess {
  response_sender: McaiChannel,
  status: JobStatus,
  worker_configuration: WorkerConfiguration,
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
    }
  }

  fn handle(&mut self, message_event: Arc<Mutex<ME>>, order_message: OrderMessage) -> Result<()> {
    let response = match order_message {
      OrderMessage::InitProcess(_job) => {
        self.status = JobStatus::Initialized;
        self.get_status_feedback()
      }
      OrderMessage::Job(job) | OrderMessage::StartProcess(job) => {
        info!("Process job: {:?}", job);
        self.status = JobStatus::Running;
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
      OrderMessage::StopProcess(_job) => {
        self.status = JobStatus::Completed;

        self.get_status_feedback()
      }
      OrderMessage::Status | OrderMessage::StopWorker => self.get_status_feedback(),
    };

    self.response_sender.lock().unwrap().send_response(response)
  }
}

impl SimpleProcess {
  fn get_status_feedback(&self) -> ResponseMessage {
    ResponseMessage::Feedback(Feedback::Status(ProcessStatus::new_with_info(
      self.status.clone(),
      SystemInformation::new(&self.worker_configuration.clone()),
    )))
  }
}

impl Drop for SimpleProcess {
  fn drop(&mut self) {
    info!("Simple process dropped with status: {:?}", self.status);
  }
}
