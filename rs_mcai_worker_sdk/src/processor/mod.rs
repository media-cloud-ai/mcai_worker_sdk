#[cfg(feature = "media")]
mod media_process;
mod simple_process;

#[cfg(feature = "media")]
use media_process::MediaProcess as ProcessEngine;
#[cfg(not(feature = "media"))]
use simple_process::SimpleProcess as ProcessEngine;

use crate::{
  message_exchange::{InternalExchange, OrderMessage, ResponseMessage},
  worker::{status::WorkerStatus, WorkerConfiguration},
  job::{Job, JobResult, JobStatus}, McaiChannel, MessageError, MessageEvent, Result,
};
use async_std::task;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{
  sync::{Arc, Mutex},
  thread::spawn,
};

pub trait Process<P, ME> {
  fn new(
    message_event: Arc<Mutex<ME>>,
    response_sender: McaiChannel,
    worker_configuration: WorkerConfiguration,
  ) -> Self;

  fn handle(&mut self, message_event: Arc<Mutex<ME>>, order_message: OrderMessage) -> Result<()>;

  fn get_current_job_id(&self, message_event: Arc<Mutex<ME>>) -> Option<u64>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessStatus {
  pub job: Option<JobResult>, // Contains job_status
  pub worker: WorkerStatus,
}

impl ProcessStatus {
  pub fn new(worker_status: WorkerStatus, job_result: Option<JobResult>) -> Self {
    ProcessStatus {
      job: job_result,
      worker: worker_status,
    }
  }
}

pub struct Processor {
  internal_exchange: Arc<dyn InternalExchange>,
  worker_configuration: WorkerConfiguration,
}

impl Processor {
  pub fn new(
    internal_exchange: Arc<dyn InternalExchange>,
    worker_configuration: WorkerConfiguration,
  ) -> Self {
    Processor {
      internal_exchange,
      worker_configuration,
    }
  }

  pub fn run<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    self,
    message_event: Arc<Mutex<ME>>,
  ) -> Result<()> {
    let order_receiver_from_exchange = self.internal_exchange.get_order_receiver();
    let response_sender_to_exchange = self.internal_exchange.get_response_sender();

    let cloned_worker_configuration = self.worker_configuration.clone();

    let thread = spawn(move || {
      // Initialize the worker
      if let Err(e) = message_event.lock().unwrap().init() {
        return Some(e);
      }

      // Create Simple or Media Process
      let mut process = ProcessEngine::new(
        message_event.clone(),
        response_sender_to_exchange.clone(),
        cloned_worker_configuration.clone(),
      );

      response_sender_to_exchange
        .lock()
        .unwrap()
        .send_response(ResponseMessage::WorkerCreated(
          Box::new(cloned_worker_configuration.clone()),
        ))
        .unwrap();

      loop {
        let order_receiver = order_receiver_from_exchange.clone();

        let next_message =
          task::block_on(async move { order_receiver.lock().unwrap().recv().await });

        if let Ok(message) = next_message {
          debug!("Processor received an order message: {:?}", message);

          let current_job_id = process.get_current_job_id(message_event.clone());

          if let Err(error) = validate_message(&message, current_job_id) {
            let response = ResponseMessage::Error(error);
            debug!(
              "Processor send the process response message: {:?}",
              response
            );
            response_sender_to_exchange
              .lock()
              .unwrap()
              .send_response(response)
              .unwrap();

            debug!("Process response message sent!");
            continue;
          }

          if let Err(error) = process.handle(message_event.clone(), message) {
            let response = ResponseMessage::Error(error);
            debug!(
              "Processor send the process response message: {:?}",
              response
            );
            response_sender_to_exchange
              .lock()
              .unwrap()
              .send_response(response)
              .unwrap();

            debug!("Process response message sent!");
          }
        }
      }
    });

    if let Some(error) = thread.join().unwrap() {
      Err(error)
    } else {
      Ok(())
    }
  }
}

fn validate_message(message: &OrderMessage, current_job_id: Option<u64>) -> Result<()> {
  match message {
    OrderMessage::InitProcess(job) => {
      if current_job_id.is_some() {
        build_error(job, "Cannot initialize this job, an another job is already in progress.")?;
      }
    }
    OrderMessage::Job(job) | OrderMessage::StartProcess(job) => {
      if current_job_id.is_none() {
        build_error(job, "Cannot start a not initialized job.")?;
      }
      if current_job_id != Some(job.job_id) {
        build_error(job, "The Job ID is not the same as the initialized job.")?;
      }
    }
    OrderMessage::StopProcess(job) => {
      if current_job_id.is_none() {
        build_error(job, "Cannot stop a non-running job.")?;
      }
      if current_job_id != Some(job.job_id) {
        build_error(job, "The Job ID is not the same as the current job.")?;
      }
    }
    _ => {}
  }
  Ok(())
}

fn build_error(job: &Job, message: &str) -> Result<()> {
  Err(MessageError::ProcessingError(
    JobResult::new(job.job_id)
      .with_status(JobStatus::Error)
      .with_message(message),
  ))
}