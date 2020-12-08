mod local;

use crate::job::{Job, JobResult};
use crate::Result;
pub use local::LocalExchange;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum ResponseMessage {
  Initialized,
  Completed(JobResult),
  Progression(u64, u8),
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderMessage {
  InitProcess(Job),
  StartProcess(Job),
  StopProcess(Job),
  StopWorker,
}

pub type SharedExternalExchange = Arc<Mutex<dyn ExternalExchange + Send>>;
pub type SharedInternalExchange = Arc<Mutex<dyn InternalExchange + Send>>;

pub trait ExternalExchange {
  fn send_order(&mut self, message: OrderMessage) -> Result<()>;
  fn next_response(&mut self) -> Result<Option<ResponseMessage>>;
}

pub trait InternalExchange {
  fn next_order(&mut self) -> Result<Option<OrderMessage>>;
  fn send_response(&mut self, message: ResponseMessage) -> Result<()>;
}
