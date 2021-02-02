//! Connectors between message brokers and processors

pub mod local;
pub mod message;
pub mod rabbitmq;

pub use {local::LocalExchange, rabbitmq::RabbitmqExchange};

use crate::prelude::*;
use async_std::channel::Receiver;
use message::{OrderMessage, ResponseMessage};
use std::sync::{Arc, Mutex};

pub type SharedExternalExchange = Arc<Mutex<dyn ExternalExchange + Send>>;
pub type SharedInternalExchange = Arc<Mutex<dyn InternalExchange + Send>>;

pub trait ExternalExchange {
  fn send_order(&mut self, message: OrderMessage) -> Result<()>;
  fn next_response(&mut self) -> Result<Option<ResponseMessage>>;
}

pub trait InternalExchange {
  fn send_response(&mut self, message: ResponseMessage) -> Result<()>;
  fn get_response_sender(&self) -> Arc<Mutex<dyn ResponseSender + Send>>;
  fn get_worker_response_sender(&self) -> McaiChannel;
  fn get_order_receiver(&self) -> Arc<Mutex<Receiver<OrderMessage>>>;
}

pub trait ResponseSender {
  fn send_response(&'_ self, message: ResponseMessage) -> Result<()>;
}

pub trait WorkerResponseSender: ResponseSender {
  fn progression(&'_ self, job_id: u64, progression: u8) -> Result<()>;
  fn is_stopped(&'_ self) -> bool;
}
