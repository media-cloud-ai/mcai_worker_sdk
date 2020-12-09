#[cfg(feature = "media")]
mod media_process;
mod simple_process;

#[cfg(feature = "media")]
use media_process::MediaProcess as ProcessEngine;
#[cfg(not(feature = "media"))]
use simple_process::SimpleProcess as ProcessEngine;

use crate::{
  job::Job,
  message_exchange::{InternalExchange, OrderMessage, ResponseMessage},
  JobResult, MessageEvent, Result,
};
use async_std::task;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{
  sync::{Arc, Mutex},
  thread::spawn,
};

pub trait Process<P, ME> {
  fn init(&mut self, message_event: Arc<Mutex<ME>>, job: &Job) -> Result<()>;

  fn start(&mut self, message_event: Arc<Mutex<ME>>, job: &Job) -> Result<JobResult>;

  fn stop(&mut self, message_event: Arc<Mutex<ME>>, job: &Job) -> Result<JobResult>;
}

pub struct Processor {
  internal_exchange: Arc<dyn InternalExchange>,
}

impl Processor {
  pub fn new(internal_exchange: Arc<dyn InternalExchange>) -> Self {
    Processor { internal_exchange }
  }

  pub fn run<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    self,
    message_event: Arc<Mutex<ME>>,
  ) -> Result<()> {
    let order_receiver = self.internal_exchange.get_order_receiver();
    let response_sender = self.internal_exchange.get_response_sender();
    // let internal_exchange = Arc::new(Mutex::new(self.internal_exchange));

    let thread = spawn(move || {
      // Initialize the worker
      if let Err(e) = message_event.lock().unwrap().init() {
        return Some(e);
      }

      // Create Simple or Media Process
      let mut process = ProcessEngine::default();

      loop {
        let order_receiver = order_receiver.clone();

        let next_message =
          task::block_on(async move { order_receiver.lock().unwrap().recv().await });

        if let Ok(message) = next_message {
          let response = match message {
            OrderMessage::InitProcess(job) => process
              .init(message_event.clone(), &job)
              .map(|_| ResponseMessage::Initialized),
            OrderMessage::StartProcess(job) => {
              info!("Process job: {:?}", job);
              process
                .start(message_event.clone(), &job)
                .map(ResponseMessage::Completed)
            }
            OrderMessage::StopProcess(job) => process
              .stop(message_event.clone(), &job)
              .map(ResponseMessage::Completed),
            OrderMessage::StopWorker => {
              break None;
            }
          };

          let response = response.map_err(ResponseMessage::Error);

          let response = match response {
            Ok(re) => re,
            Err(re) => re,
          };

          response_sender
            .lock()
            .unwrap()
            .send_response(response)
            .unwrap();
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
