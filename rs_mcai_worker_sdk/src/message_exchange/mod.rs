mod local;

use crate::Result;
use crate::job::Job;
pub use local::LocalExchange;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum ResponseMessage {
  Completed,
  Progression(u8),
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderMessage {
  Job(Job),
  Stop,
}

pub type SharedExternalExchange = Arc<Mutex<dyn InternalExchange + Send>>;
pub type SharedInternalExchange = Arc<Mutex<dyn InternalExchange + Send>>;

pub trait ExternalExchange {
  fn send_order(&mut self, message: OrderMessage) -> Result<()>;
  fn next_response(&mut self) -> Result<Option<ResponseMessage>>;
}

pub trait InternalExchange {
  fn next_order(&mut self) -> Result<Option<OrderMessage>>;
  fn send_response(&mut self, message: ResponseMessage) -> Result<()>;
}
