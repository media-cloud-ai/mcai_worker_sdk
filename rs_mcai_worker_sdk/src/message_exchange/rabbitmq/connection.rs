use super::{channels::declare_consumer_channel, RabbitmqConsumer};
use crate::{
  config,
  message_exchange::{OrderMessage, ResponseMessage},
  worker::WorkerConfiguration,
  Result,
};
use async_amqp::*;
use async_std::channel::Sender;
use lapin::{Connection, ConnectionProperties};

pub struct RabbitmqConnection {
  job_consumer: RabbitmqConsumer,
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

    let job_consumer =
      RabbitmqConsumer::new(&channel, order_sender.clone(), &queue_name, "amqp_worker").await?;

    let queue_name = worker_configuration.get_direct_messaging_queue_name();

    RabbitmqConsumer::new(&channel, order_sender, &queue_name, "status_amqp_worker").await?;

    Ok(RabbitmqConnection { job_consumer })
  }

  pub async fn send_response(&mut self, response: ResponseMessage) -> Result<()> {
    self.job_consumer.send_response(response).await;

    Ok(())
  }
}
