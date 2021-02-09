use super::{
  channels::declare_consumer_channel, consumer::RABBITMQ_CONSUMER_TAG_DIRECT,
  consumer::RABBITMQ_CONSUMER_TAG_JOB, CurrentOrders, RabbitmqConsumer, RabbitmqPublisher,
};
use crate::{
  config,
  message_exchange::{OrderMessage, ResponseMessage},
  worker::WorkerConfiguration,
  Result,
};
use async_amqp::*;
use async_std::channel::Sender;
use lapin::{Connection, ConnectionProperties};
use std::sync::{Arc, Mutex};

pub struct RabbitmqConnection {
  _job_consumer: RabbitmqConsumer,
  _order_consumer: RabbitmqConsumer,
  response_publisher: RabbitmqPublisher,
  current_orders: Arc<Mutex<CurrentOrders>>,
}

impl RabbitmqConnection {
  pub async fn new(
    worker_configuration: &WorkerConfiguration,
    order_sender: Sender<OrderMessage>,
  ) -> Result<Self> {
    let amqp_uri = config::get_amqp_uri();
    let properties = ConnectionProperties::default()
      .with_default_executor(8)
      .with_async_std();

    let connection = Connection::connect_uri(amqp_uri, properties).await?;

    log::info!("Connected to RabbitMQ");

    let channel = declare_consumer_channel(&connection, worker_configuration);

    let queue_name = worker_configuration.get_queue_name();

    let current_orders = CurrentOrders::default();
    let current_orders = Arc::new(Mutex::new(current_orders));

    let job_consumer = RabbitmqConsumer::new(
      &channel,
      order_sender.clone(),
      &queue_name,
      RABBITMQ_CONSUMER_TAG_JOB,
      current_orders.clone(),
    )
    .await?;

    let queue_name = worker_configuration.get_direct_messaging_queue_name();

    let order_consumer = RabbitmqConsumer::new(
      &channel,
      order_sender,
      &queue_name,
      RABBITMQ_CONSUMER_TAG_DIRECT,
      current_orders.clone(),
    )
    .await?;

    let response_publisher = RabbitmqPublisher::new(&channel, current_orders.clone()).await?;

    Ok(RabbitmqConnection {
      _job_consumer: job_consumer,
      _order_consumer: order_consumer,
      response_publisher,
      current_orders,
    })
  }

  pub async fn send_response(&mut self, response: ResponseMessage) -> Result<()> {
    self
      .response_publisher
      .send_response(response.clone())
      .await;

    Ok(())
  }

  pub fn get_current_orders(&self) -> Arc<Mutex<CurrentOrders>> {
    self.current_orders.clone()
  }
}

impl Drop for RabbitmqConnection {
  fn drop(&mut self) {
    // TODO close consumer/publisher connections
    self
      .current_orders
      .lock()
      .unwrap()
      .reset_process_deliveries();
    self
      .current_orders
      .lock()
      .unwrap()
      .reset_status_deliveries();
  }
}
