use super::{channels::declare_consumer_channel, RabbitmqConsumer, RabbitmqPublisher};
use crate::{
  config,
  message_exchange::{OrderMessage, ResponseMessage},
  worker::WorkerConfiguration,
  Result,
};
use async_amqp::*;
use async_std::channel::Sender;
use failure::_core::fmt::{Display, Formatter};
use lapin::message::Delivery;
use lapin::{Connection, ConnectionProperties};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurrentOrders {
  pub init: Option<Delivery>,
  pub start: Option<Delivery>,
  pub stop: Option<Delivery>,
  pub status: Option<Delivery>,
}

impl CurrentOrders {
  pub(crate) fn reset_process_deliveries(&mut self) {
    self.init = None;
    self.start = None;
    self.stop = None;
  }

  pub(crate) fn reset_status_deliveries(&mut self) {
    self.status = None;
  }

  pub(crate) fn get_process_deliveries(&self) -> Vec<Delivery> {
    Self::filter_sort_and_dedup_deliveries(vec![
      self.init.clone(),
      self.start.clone(),
      self.stop.clone(),
    ])
  }

  pub(crate) fn get_status_deliveries(&self) -> Vec<Delivery> {
    Self::filter_sort_and_dedup_deliveries(vec![self.status.clone()])
  }

  fn filter_sort_and_dedup_deliveries(deliveries: Vec<Option<Delivery>>) -> Vec<Delivery> {
    // Filter nones
    let mut deliveries: Vec<Delivery> = deliveries
      .iter()
      .cloned()
      .filter(|delivery| delivery.is_some())
      .map(|delivery| delivery.unwrap())
      .collect();

    // Sort deliveries by tag
    deliveries.sort_by(|a, b| a.delivery_tag.partial_cmp(&b.delivery_tag).unwrap());

    // Remove duplicated deliveries
    deliveries.dedup();

    deliveries
  }
}

impl Display for CurrentOrders {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
    write!(
      f,
      "CurrentOrders {{ init: {:?}, start: {:?}, stop: {:?}, status: {:?} }}",
      self.init.clone().map(|d| d.delivery_tag),
      self.start.clone().map(|d| d.delivery_tag),
      self.stop.clone().map(|d| d.delivery_tag),
      self.status.clone().map(|d| d.delivery_tag)
    )
  }
}

pub struct RabbitmqConnection {
  _job_consumer: RabbitmqConsumer,
  _order_consumer: RabbitmqConsumer,
  response_publisher: RabbitmqPublisher,
  _current_orders: Arc<Mutex<CurrentOrders>>,
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
      "amqp_worker",
      current_orders.clone(),
    )
    .await?;

    let queue_name = worker_configuration.get_direct_messaging_queue_name();

    let order_consumer = RabbitmqConsumer::new(
      &channel,
      order_sender,
      &queue_name,
      "status_amqp_worker",
      current_orders.clone(),
    )
    .await?;

    let response_publisher = RabbitmqPublisher::new(&channel, current_orders.clone()).await?;

    Ok(RabbitmqConnection {
      _job_consumer: job_consumer,
      _order_consumer: order_consumer,
      response_publisher,
      _current_orders: current_orders,
    })
  }

  pub async fn send_response(&mut self, response: ResponseMessage) -> Result<()> {
    self
      .response_publisher
      .send_response(response.clone())
      .await;

    Ok(())
  }
}
