use super::RabbitmqConnection;
use crate::{
  message_exchange::{InternalExchange, OrderMessage, ResponseMessage, ResponseSender},
  worker::WorkerConfiguration,
  Result,
};
use async_std::{
  channel::{self, Receiver},
  task,
};
use std::sync::{Arc, Mutex};

pub struct RabbitmqExchange {
  connection: Arc<Mutex<RabbitmqConnection>>,
  order_receiver: Arc<Mutex<Receiver<OrderMessage>>>,
}

impl RabbitmqExchange {
  pub async fn new(worker_configuration: &WorkerConfiguration) -> Result<Self> {
    let connection = RabbitmqConnection::new(worker_configuration).await?;
    let connection = Arc::new(Mutex::new(connection));
    let (order_sender, order_receiver) = channel::unbounded();

    let order_receiver = Arc::new(Mutex::new(order_receiver));

    connection
      .lock()
      .unwrap()
      .bind_consumer(
        order_sender.clone(),
        &worker_configuration.get_queue_name(),
        "amqp_worker",
      )
      .await?;

    connection
      .lock()
      .unwrap()
      .bind_consumer(
        order_sender,
        &worker_configuration.get_direct_messaging_queue_name(),
        "status_amqp_worker",
      )
      .await?;

    Ok(RabbitmqExchange {
      connection,
      order_receiver,
    })
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
