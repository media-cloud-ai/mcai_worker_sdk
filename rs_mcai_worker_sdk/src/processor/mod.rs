use crate::message_exchange::{OrderMessage, ResponseMessage, SharedInternalExchange};
use crate::{JobResult, MessageEvent, Result};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::thread::spawn;

pub struct Processor {
  internal_exchange: SharedInternalExchange,
}

impl Processor {
  pub fn new(internal_exchange: SharedInternalExchange) -> Self {
    Processor { internal_exchange }
  }

  pub fn run<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    self,
    mut message_event: ME,
  ) -> Result<()> {
    let internal_exchange = self.internal_exchange;

    let thread = spawn(move || {
      if let Err(e) = message_event.init() {
        return Some(e);
      }

      loop {
        let mut cloned_internal_exchange = internal_exchange.lock().unwrap();
        if let Ok(Some(message)) = cloned_internal_exchange.next_order() {
          match message {
            OrderMessage::Stop => {
              break None;
            }
            OrderMessage::Job(job) => {
              log::info!("New job: {:?}", job);

              let result = message_event.process(
                // internal_exchange.clone(),
                None,
                job.get_parameters().unwrap(),
                JobResult::from(job),
              );

              if let Err(e) = result {
                return Some(e);
              } else {
                cloned_internal_exchange
                  .send_response(ResponseMessage::Completed)
                  .unwrap();
              }
            }
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
