use super::RabbitmqConnection;
use crate::{
  message_exchange::{
    InternalExchange, OrderMessage, ResponseMessage, ResponseSender, WorkerResponseSender,
  },
  prelude::*,
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
    let (order_sender, order_receiver) = channel::unbounded();

    let connection = RabbitmqConnection::new(worker_configuration, order_sender).await?;
    let connection = Arc::new(Mutex::new(connection));

    let order_receiver = Arc::new(Mutex::new(order_receiver));

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

  fn get_worker_response_sender(&self) -> McaiChannel {
    Arc::new(Mutex::new(RabbitmqResponseSender {
      connection: self.connection.clone(),
    }))
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

impl WorkerResponseSender for RabbitmqResponseSender {
  fn progression(&'_ self, job_id: u64, progression: u8) -> Result<()> {
    let message = ResponseMessage::Feedback(Feedback::Progression(JobProgression::new(
      job_id,
      progression,
    )));
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

  fn is_stopped(&self) -> bool {
    self
      .connection
      .lock()
      .unwrap()
      .get_current_orders()
      .lock()
      .unwrap()
      .stop
      .is_some()
  }
}
