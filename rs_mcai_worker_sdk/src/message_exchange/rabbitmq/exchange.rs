use super::RabbitmqConnection;
use crate::{
  message_exchange::{InternalExchange, OrderMessage, ResponseMessage, ResponseSender},
  worker::WorkerConfiguration,
  Result, SdkResult,
};
use async_std::{
  channel::{self, Receiver, Sender},
  task,
};
use std::sync::{Arc, Mutex};

pub struct RabbitmqExchange {
  connection: Arc<Mutex<RabbitmqConnection>>,
  order_sender: Sender<OrderMessage>,
  order_receiver: Arc<Mutex<Receiver<OrderMessage>>>,
}

impl RabbitmqExchange {
  pub async fn new(worker_configuration: &WorkerConfiguration) -> SdkResult<Self> {
    let connection = RabbitmqConnection::new(worker_configuration).await?;
    let connection = Arc::new(Mutex::new(connection));
    let (order_sender, order_receiver) = channel::unbounded();

    let order_receiver = Arc::new(Mutex::new(order_receiver));

    Ok(RabbitmqExchange {
      connection,
      order_sender,
      order_receiver,
    })
  }

  pub async fn bind_consumer(&mut self, queue_name: &str, consumer_tag: &str) -> SdkResult<()> {
    self
      .connection
      .lock()
      .unwrap()
      .bind_consumer(self.order_sender.clone(), queue_name, consumer_tag)
      .await
  }
}

impl InternalExchange for RabbitmqExchange {
  fn send_response(&mut self, response: ResponseMessage) -> Result<()> {
    task::block_on(async move {
      self
        .connection
        .lock()
        .unwrap()
        .send_response(response)
        .await
    })
  }

  fn get_response_sender(&self) -> Arc<Mutex<dyn ResponseSender + Send>> {
    let connection = self.connection.clone();
    Arc::new(Mutex::new(RabbitmqResponseSender { connection }))
  }

  fn get_order_receiver(&self) -> Arc<Mutex<Receiver<OrderMessage>>> {
    self.order_receiver.clone()
  }
}

struct RabbitmqResponseSender {
  connection: Arc<Mutex<RabbitmqConnection>>,
}

impl ResponseSender for RabbitmqResponseSender {
  fn send_response(&'_ self, message: ResponseMessage) -> Result<()> {
    task::block_on(async move {
      self
        .connection
        .lock()
        .unwrap()
        .send_response(message)
        .await
        .unwrap()
    });
    Ok(())
  }
}
