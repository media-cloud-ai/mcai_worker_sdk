#[cfg(feature = "media")]
mod media_process;
mod simple_process;

#[cfg(feature = "media")]
use media_process::MediaProcess as ProcessEngine;
#[cfg(not(feature = "media"))]
use simple_process::SimpleProcess as ProcessEngine;

use crate::job::JobStatus;
use crate::worker::system_information::SystemInformation;
use crate::worker::WorkerConfiguration;
use crate::{
  message_exchange::{InternalExchange, OrderMessage, ResponseMessage},
  JobResult, McaiChannel, MessageEvent, Result,
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessStatus {
  pub job_status: JobStatus,
  pub job_result: Option<JobResult>,
  pub worker_status: Option<SystemInformation>,
}

impl ProcessStatus {
  pub fn new(job_status: JobStatus) -> Self {
    ProcessStatus {
      job_status,
      job_result: None,
      worker_status: None,
    }
  }

  pub fn new_with_result(job_status: JobStatus, job_result: JobResult) -> Self {
    ProcessStatus {
      job_status,
      job_result: Some(job_result),
      worker_status: None,
    }
  }

  pub fn new_with_info(job_status: JobStatus, system_info: SystemInformation) -> Self {
    ProcessStatus {
      job_status,
      job_result: None,
      worker_status: Some(system_info),
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

      loop {
        let order_receiver = order_receiver_from_exchange.clone();

        let next_message =
          task::block_on(async move { order_receiver.lock().unwrap().recv().await });

        if let Ok(message) = next_message {
          debug!("Processor received an order message: {:?}", message);

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
