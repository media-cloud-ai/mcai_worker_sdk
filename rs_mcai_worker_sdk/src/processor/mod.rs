#[cfg(feature = "media")]
mod media_process;
mod simple_process;
mod process;
mod process_status;

#[cfg(feature = "media")]
use media_process::MediaProcess as ProcessEngine;
#[cfg(not(feature = "media"))]
use simple_process::SimpleProcess as ProcessEngine;

use crate::{
  message_exchange::{
    message::ResponseMessage,
    InternalExchange,
  },
  worker::WorkerConfiguration,
  MessageEvent, Result,
};
use async_std::task;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{
  sync::{Arc, Mutex},
  thread::spawn,
};

pub use process::Process;
pub use process_status::ProcessStatus;

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
    let worker_response_sender_to_exchange = self.internal_exchange.get_worker_response_sender();

    let cloned_worker_configuration = self.worker_configuration.clone();

    let thread = spawn(move || {
      // Initialize the worker
      if let Err(e) = message_event.lock().unwrap().init() {
        return Some(e);
      }

      // Create Simple or Media Process
      let mut process = ProcessEngine::new(
        message_event.clone(),
        worker_response_sender_to_exchange,
        cloned_worker_configuration.clone(),
      );

      response_sender_to_exchange
        .lock()
        .unwrap()
        .send_response(ResponseMessage::WorkerCreated(Box::new(
          cloned_worker_configuration.clone(),
        )))
        .unwrap();

      loop {
        let order_receiver = order_receiver_from_exchange.clone();

        let next_message = task::block_on(async move {
          order_receiver.lock().unwrap().recv().await
        });

        if let Ok(message) = next_message {
          log::debug!("Processor received an order message: {:?}", message);

          let current_job_id = process.get_current_job_id(message_event.clone());

          if let Err(error) = message.matches_job_id(current_job_id) {
            let response = ResponseMessage::Error(error);
            log::debug!(
              "Processor send the process response message: {:?}",
              response
            );
            response_sender_to_exchange
              .lock()
              .unwrap()
              .send_response(response)
              .unwrap();

            log::debug!("Process response message sent!");
            continue;
          }

          // process the message on the processor
          let response = process.handle(message_event.clone(), message);          

          // manage errors returned by the processor
          if let Err(error) = response {
            let response = ResponseMessage::Error(error);
            log::debug!(
              "Processor send the process response message: {:?}",
              response
            );
            response_sender_to_exchange
              .lock()
              .unwrap()
              .send_response(response)
              .unwrap();

            log::debug!("Process response message sent!");
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
