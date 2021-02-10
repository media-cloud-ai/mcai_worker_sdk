use futures_util::StreamExt;
use lapin::{
  options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions},
  types::{AMQPValue, FieldTable},
  BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
};
use crate::{
  config,
  message_exchange::rabbitmq::{
    channels::{BindDescription, ExchangeDescription, QueueDescription},
    EXCHANGE_NAME_DIRECT_MESSAGING, EXCHANGE_NAME_WORKER_RESPONSE, QUEUE_WORKER_CREATED,
    QUEUE_WORKER_INITIALIZED, QUEUE_WORKER_STARTED, QUEUE_WORKER_STATUS, QUEUE_WORKER_TERMINATED,
    QUEUE_WORKER_UPDATED, WORKER_RESPONSE_NOT_FOUND,
  },
  prelude::*,
};
use std::{collections::HashMap, sync::mpsc::Sender};

pub struct AmqpConnection {
  channel: Channel,
}

impl AmqpConnection {
  pub fn new() -> Result<Self> {
    let amqp_uri = config::get_amqp_uri();

    let connection = Connection::connect_uri(
      amqp_uri,
      ConnectionProperties::default().with_default_executor(8),
    )
    .wait()?;

    let channel = connection.create_channel().wait()?;

    Self::declare_consumed_queues(&channel);

    Ok(AmqpConnection { channel })
  }

  pub fn start_consumer<T: 'static + serde::de::DeserializeOwned + Send>(
    &self,
    queue_name: &str,
    sender: Sender<T>,
  ) {
    let channel = self.channel.clone();
    let sender = sender.clone();
    let queue_name = queue_name.to_string();

    std::thread::spawn(move || {
      let mut status_consumer = channel
        .basic_consume(
          &queue_name,
          &format!("test_consumer_{}", queue_name),
          BasicConsumeOptions::default(),
          FieldTable::default(),
        )
        .wait()
        .unwrap();

      while let Some(delivery) = futures_executor::block_on(status_consumer.next()) {
        if let Ok((channel, delivery)) = delivery {
          let message_data = std::str::from_utf8(&delivery.data).unwrap();
          log::debug!("Consuming message: {:?}", message_data);

          let response_message = serde_json::from_str(message_data).unwrap();
          sender.send(response_message).unwrap();

          channel
            .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
            .wait()
            .unwrap();
        }
      }
    });
  }

  pub fn send_order(&self, worker_ids: Vec<&str>, order_message: &OrderMessage) -> Result<()> {
    let status_message = serde_json::to_string(&order_message).unwrap();

    if worker_ids.is_empty() {
      let mut headers = FieldTable::default();
      headers.insert("broadcast".into(), AMQPValue::Boolean(true));

      self
        .channel
        .basic_publish(
          EXCHANGE_NAME_DIRECT_MESSAGING,
          "mcai_workers_status",
          BasicPublishOptions::default(),
          status_message.as_bytes().to_vec(),
          BasicProperties::default().with_headers(headers),
        )
        .wait()?;

      return Ok(());
    }

    for worker_id in worker_ids {
      let mut headers = FieldTable::default();
      headers.insert(
        "worker_name".into(),
        AMQPValue::LongString(worker_id.to_string().into()),
      );

      self
        .channel
        .basic_publish(
          EXCHANGE_NAME_DIRECT_MESSAGING,
          "mcai_workers_status",
          BasicPublishOptions::default(),
          status_message.as_bytes().to_vec(),
          BasicProperties::default().with_headers(headers),
        )
        .wait()?;
    }

    Ok(())
  }

  fn declare_consumed_queues(channel: &Channel) {
    ExchangeDescription::new(EXCHANGE_NAME_WORKER_RESPONSE, ExchangeKind::Topic)
      .with_alternate_exchange(WORKER_RESPONSE_NOT_FOUND)
      .declare(channel);

    ExchangeDescription::new(EXCHANGE_NAME_JOB_RESPONSE, ExchangeKind::Topic)
      .with_alternate_exchange(JOB_RESPONSE_NOT_FOUND)
      .declare(channel);

    Self::declare_queue(channel, EXCHANGE_NAME_WORKER_RESPONSE, QUEUE_WORKER_CREATED);
    Self::declare_queue(
      channel,
      EXCHANGE_NAME_WORKER_RESPONSE,
      QUEUE_WORKER_INITIALIZED,
    );
    Self::declare_queue(channel, EXCHANGE_NAME_WORKER_RESPONSE, QUEUE_WORKER_STARTED);
    Self::declare_queue(channel, EXCHANGE_NAME_WORKER_RESPONSE, QUEUE_WORKER_STATUS);
    Self::declare_queue(
      channel,
      EXCHANGE_NAME_WORKER_RESPONSE,
      QUEUE_WORKER_TERMINATED,
    );
    Self::declare_queue(channel, EXCHANGE_NAME_WORKER_RESPONSE, QUEUE_WORKER_UPDATED);

    Self::declare_queue(channel, EXCHANGE_NAME_JOB_RESPONSE, QUEUE_JOB_COMPLETED);
    Self::declare_queue(channel, EXCHANGE_NAME_JOB_RESPONSE, QUEUE_JOB_PROGRESSION);
    Self::declare_queue(channel, EXCHANGE_NAME_JOB_RESPONSE, QUEUE_JOB_STOPPED);
  }

  fn declare_queue(channel: &Channel, exchange: &str, queue: &str) {
    QueueDescription {
      name: queue.to_string(),
      durable: true,
      ..Default::default()
    }
    .declare(&channel);

    BindDescription {
      exchange: exchange.to_string(),
      queue: queue.to_string(),
      routing_key: queue.to_string(),
      headers: HashMap::new(),
    }
    .declare(&channel);
  }
}
