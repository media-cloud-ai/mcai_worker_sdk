use super::{helpers, publish, CurrentOrders};
use crate::{message_exchange::OrderMessage, MessageError, Result};
use amq_protocol_types::FieldTable;
use async_std::{
  channel::Sender,
  stream::StreamExt,
  task::{self, JoinHandle},
};
use lapin::{
  message::Delivery,
  options::{BasicConsumeOptions, BasicRejectOptions},
  Channel,
};
use std::{
  convert::TryFrom,
  sync::{Arc, Mutex},
};

pub struct RabbitmqConsumer {
  handle: Option<JoinHandle<()>>,
}

impl RabbitmqConsumer {
  pub async fn new(
    channel: &Channel,
    sender: Sender<OrderMessage>,
    queue_name: &str,
    consumer_tag: &str,
    current_orders: Arc<Mutex<CurrentOrders>>,
  ) -> Result<Self> {
    let mut consumer = channel
      .basic_consume(
        queue_name,
        consumer_tag,
        BasicConsumeOptions::default(),
        FieldTable::default(),
      )
      .await?;

    let channel = Arc::new(channel.clone());
    let queue_name_clone = queue_name.to_string();

    let handle = Some(task::spawn(async move {
      while let Some(delivery) = consumer.next().await {
        let (_, delivery) = delivery.expect("error in consumer");

        if let Err(error) = Self::process_delivery(
          sender.clone(),
          channel.clone(),
          &delivery,
          &queue_name_clone.clone(),
          current_orders.clone(),
        )
        .await
        {
          log::error!("{:?}", error);
          if let Err(error) = publish::error(channel.clone(), &delivery, &error).await {
            log::error!("Unable to publish response: {:?}", error);
          }
        }
      }
    }));

    Ok(RabbitmqConsumer { handle })
  }

  async fn process_delivery(
    sender: Sender<OrderMessage>,
    channel: Arc<Channel>,
    delivery: &Delivery,
    queue_name: &str,
    current_orders: Arc<Mutex<CurrentOrders>>,
  ) -> Result<()> {
    let count = helpers::get_message_death_count(&delivery);
    let message_data = std::str::from_utf8(&delivery.data).map_err(|e| {
      MessageError::RuntimeError(format!("unable to retrieve raw message: {:?}", e))
    })?;

    let order_message = OrderMessage::try_from(message_data)?;

    log::debug!(
      "RabbitMQ consumer on {:?} queue received message: {:?} (iteration: {})",
      queue_name,
      order_message,
      count.unwrap_or(0)
    );

    match order_message {
      OrderMessage::Job(_) => {
        let is_initializing = current_orders.lock().unwrap().init.is_some();
        let is_starting = current_orders.lock().unwrap().start.is_some();

        if is_initializing || is_starting {
          // Worker already processing
          return Self::reject_delivery(channel, delivery.delivery_tag).await;
        }

        current_orders.lock().unwrap().init = Some(delivery.clone());
        current_orders.lock().unwrap().start = Some(delivery.clone());
      }
      OrderMessage::InitProcess(_) => {
        if current_orders.lock().unwrap().init.is_some() {
          // Worker already processing
          return Self::reject_delivery(channel, delivery.delivery_tag).await;
        }

        current_orders.lock().unwrap().init = Some(delivery.clone());
      }
      OrderMessage::StartProcess(_) => {
        if current_orders.lock().unwrap().start.is_some() {
          // Worker already processing
          return Self::reject_delivery(channel, delivery.delivery_tag).await;
        }

        current_orders.lock().unwrap().start = Some(delivery.clone());
      }
      OrderMessage::StopProcess(_) | OrderMessage::StopWorker => {
        if current_orders.lock().unwrap().stop.is_some() {
          // Worker already stopping
          return Self::reject_delivery(channel, delivery.delivery_tag).await;
        }

        current_orders.lock().unwrap().stop = Some(delivery.clone());
      }
      OrderMessage::Status => {
        if current_orders.lock().unwrap().status.is_some() {
          // Worker already checking status
          return Self::reject_delivery(channel, delivery.delivery_tag).await;
        }

        current_orders.lock().unwrap().status = Some(delivery.clone());
      }
    }

    log::debug!(
      "RabbitMQ consumer on {:?} queue forwards the order message: {:?}",
      queue_name,
      order_message
    );
    sender.send(order_message.clone()).await.map_err(|e| {
      MessageError::RuntimeError(format!("unable to send {:?} order: {:?}", order_message, e))
    })?;

    log::debug!("Order message sent!");

    Ok(())
  }

  async fn reject_delivery(channel: Arc<Channel>, delivery_tag: u64) -> Result<()> {
    channel
      .basic_reject(delivery_tag, BasicRejectOptions { requeue: true })
      .await
      .map_err(MessageError::Amqp)
  }
}

impl Drop for RabbitmqConsumer {
  fn drop(&mut self) {
    self.handle.take().map(JoinHandle::cancel);
  }
}
