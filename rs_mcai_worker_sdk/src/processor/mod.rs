#[cfg(feature = "media")]
mod media_process;
mod simple_process;

use crate::job::Job;
use crate::message_exchange::{OrderMessage, ResponseMessage, SharedInternalExchange};
use crate::{JobResult, MessageEvent, Result};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread::spawn;

pub trait Process {
  fn init<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Rc<RefCell<ME>>,
    job: &Job,
  ) -> Result<()>;
  fn start<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Rc<RefCell<ME>>,
    job: &Job,
  ) -> Result<JobResult>;
  fn stop<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Rc<RefCell<ME>>,
    job: &Job,
  ) -> Result<JobResult>;
}

pub struct Processor {
  internal_exchange: SharedInternalExchange,
}

impl Processor {
  pub fn new(internal_exchange: SharedInternalExchange) -> Self {
    Processor { internal_exchange }
  }

  pub fn run<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    self,
    message_event: ME,
  ) -> Result<()> {
    let internal_exchange = self.internal_exchange;

    let thread = spawn(move || {
      let message_event_ref = Rc::new(RefCell::new(message_event));

      // Initialize worker
      if let Err(e) = message_event_ref.borrow_mut().init() {
        return Some(e);
      }

      #[cfg(not(feature = "media"))]
      let mut process = simple_process::SimpleProcess::default();
      #[cfg(feature = "media")]
      let mut process = media_process::MediaProcess::default();

      loop {
        let mut cloned_internal_exchange = internal_exchange.lock().unwrap();
        if let Ok(Some(message)) = cloned_internal_exchange.next_order() {
          match message {
            OrderMessage::InitProcess(job) => match process.init(message_event_ref.clone(), &job) {
              Ok(_) => cloned_internal_exchange
                .send_response(ResponseMessage::Initialized)
                .unwrap(),
              Err(e) => return Some(e),
            },
            OrderMessage::StartProcess(job) => {
              info!("Process job: {:?}", job);

              match process.start(message_event_ref.clone(), &job) {
                Ok(job_result) => cloned_internal_exchange
                  .send_response(ResponseMessage::Completed(job_result))
                  .unwrap(),
                Err(e) => return Some(e),
              }
            }
            OrderMessage::StopProcess(job) => match process.stop(message_event_ref.clone(), &job) {
              Ok(job_result) => cloned_internal_exchange
                .send_response(ResponseMessage::Completed(job_result))
                .unwrap(),
              Err(e) => return Some(e),
            },
            OrderMessage::StopWorker => {
              break None;
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
