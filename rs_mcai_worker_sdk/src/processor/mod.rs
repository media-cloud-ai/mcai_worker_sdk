#[cfg(feature = "media")]
mod media_process;
mod simple_process;

#[cfg(feature = "media")]
use media_process::MediaProcess as ProcessEngine;
#[cfg(not(feature = "media"))]
use simple_process::SimpleProcess as ProcessEngine;

use crate::{
  message_exchange::{InternalExchange, OrderMessage, ResponseMessage},
  McaiChannel, MessageEvent, Result,
};
use async_std::task;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{
  sync::{Arc, Mutex},
  thread::spawn,
};

pub trait Process<P, ME> {
  fn new(message_event: Arc<Mutex<ME>>, response_sender: McaiChannel) -> Self;

  fn handle(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    order_message: OrderMessage,
  ) -> ResponseMessage;
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

    let thread = spawn(move || {
      // Initialize the worker
      if let Err(e) = message_event.lock().unwrap().init() {
        return Some(e);
      }

      // Create Simple or Media Process
      let mut process = ProcessEngine::new(message_event.clone(), response_sender.clone());

      loop {
        let order_receiver = order_receiver.clone();

        let next_message =
          task::block_on(async move { order_receiver.lock().unwrap().recv().await });

        if let Ok(message) = next_message {
          if message == OrderMessage::StopWorker {
            break None;
          }

          let response = process.handle(message_event.clone(), message);

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
