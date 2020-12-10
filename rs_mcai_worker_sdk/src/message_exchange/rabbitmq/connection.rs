use super::RabbitmqConsumer;
use crate::{
  config,
  message_exchange::{OrderMessage, ResponseMessage},
  worker::WorkerConfiguration,
  Result,
};
use async_amqp::*;
use async_std::channel::{Receiver, Sender};
use lapin::{Channel, Connection, ConnectionProperties};

use crate::channels::declare_consumer_channel;
use crate::message_exchange::rabbitmq::publish::feedback_publisher::FeedbackPublisher;
use crate::message_exchange::Feedback;

pub struct RabbitmqConnection {
  channel: Channel,
  consumers: Vec<RabbitmqConsumer>,
  feedback_publisher: Option<FeedbackPublisher>,
}

impl RabbitmqConnection {
  pub async fn new(worker_configuration: &WorkerConfiguration) -> Result<Self> {
    let amqp_uri = config::get_amqp_uri();
    let properties = ConnectionProperties::default()
      .with_default_executor(8)
      .with_async_std();

    let connection = Connection::connect_uri(amqp_uri, properties).await?;

    log::info!("Connected to RabbitMQ");

    let channel = declare_consumer_channel(&connection, worker_configuration);

    Ok(RabbitmqConnection {
      channel,
      consumers: vec![],
      feedback_publisher: None,
    })
  }

  pub async fn bind_consumer(
    &mut self,
    sender: Sender<OrderMessage>,
    queue_name: &str,
    consumer_tag: &str,
  ) -> Result<()> {
    let consumer = RabbitmqConsumer::new(&self.channel, sender, queue_name, consumer_tag).await?;

    self.consumers.push(consumer);

    Ok(())
  }

  pub async fn send_response(&mut self, response: ResponseMessage) -> Result<()> {
    self
      .consumers
      .first()
      .unwrap()
      .send_response(response)
      .await;

    Ok(())
  }

  pub fn bind_feedback_publisher(&mut self, receiver: Receiver<Feedback>) -> Result<()> {
    self.feedback_publisher = Some(FeedbackPublisher::new(&self.channel, receiver)?);

    Ok(())
  }
}
