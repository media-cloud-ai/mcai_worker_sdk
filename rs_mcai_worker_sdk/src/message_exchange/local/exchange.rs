use crate::{
  message_exchange::{
    ExternalExchange, InternalExchange, OrderMessage, ResponseMessage, ResponseSender,
  },
  Result,
};
use async_std::{
  channel::{self, Receiver, Sender},
  task,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct LocalExchange {
  order_sender: Sender<OrderMessage>,
  order_receiver: Arc<Mutex<Receiver<OrderMessage>>>,
  response_sender: Sender<ResponseMessage>,
  response_receiver: Arc<Mutex<Receiver<ResponseMessage>>>,
}

impl LocalExchange {
  pub fn new() -> Self {
    let (order_sender, order_receiver) = channel::unbounded();
    let (response_sender, response_receiver) = channel::unbounded();

    LocalExchange {
      order_sender,
      order_receiver: Arc::new(Mutex::new(order_receiver)),
      response_sender,
      response_receiver: Arc::new(Mutex::new(response_receiver)),
    }
  }

  pub fn new_safe() -> Arc<Mutex<Self>> {
    Arc::new(Mutex::new(Self::new()))
  }
}

impl Default for LocalExchange {
  fn default() -> Self {
    Self::new()
  }
}

impl ExternalExchange for LocalExchange {
  fn send_order(&mut self, message: OrderMessage) -> Result<()> {
    task::block_on(async move { self.order_sender.send(message).await.unwrap() });
    Ok(())
  }

  fn next_response(&mut self) -> Result<Option<ResponseMessage>> {
    Ok(task::block_on(async move {
      self.response_receiver.lock().unwrap().recv().await.ok()
    }))
  }
}

impl InternalExchange for LocalExchange {
  fn get_order_receiver(&self) -> Arc<Mutex<Receiver<OrderMessage>>> {
    self.order_receiver.clone()
  }

  fn get_response_sender(&self) -> Arc<Mutex<dyn ResponseSender + Send>> {
    Arc::new(Mutex::new(LocalResponseSender {
      response_sender: self.response_sender.clone(),
    }))
  }

  fn send_response(&mut self, message: ResponseMessage) -> Result<()> {
    task::block_on(async move { self.response_sender.send(message).await.unwrap() });
    Ok(())
  }
}

struct LocalResponseSender {
  response_sender: Sender<ResponseMessage>,
}

impl ResponseSender for LocalResponseSender {
  fn send_response(&'_ self, message: ResponseMessage) -> Result<()> {
    task::block_on(async move { self.response_sender.send(message).await.unwrap() });
    Ok(())
  }
}
