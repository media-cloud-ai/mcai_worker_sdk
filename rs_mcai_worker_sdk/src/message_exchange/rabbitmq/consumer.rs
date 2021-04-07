use super::{helpers, publish, CurrentOrders};
use crate::{message_exchange::OrderMessage, MessageError, Result};
use amq_protocol_types::FieldTable;
use async_std::{
  channel::{Receiver, Sender},
  task::{self, JoinHandle},
};
use futures_util::stream::TryStreamExt;
use lapin::{
  message::Delivery,
  options::{BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicRejectOptions},
  Channel,
};
use std::{
  convert::TryFrom,
  sync::{Arc, Mutex},
  time::Duration,
};

pub struct RabbitmqConsumer {
  handle: Option<JoinHandle<()>>,
}

pub const RABBITMQ_CONSUMER_TAG_JOB: &str = "amqp_worker";
pub const RABBITMQ_CONSUMER_TAG_DIRECT: &str = "status_amqp_worker";

impl RabbitmqConsumer {
  pub async fn new(
    channel: &Channel,
    sender: Sender<OrderMessage>,
    queue_name: &str,
    consumer_tag: &str,
    current_orders: Arc<Mutex<CurrentOrders>>,
    consumer_notification_sender: Option<Sender<OrderMessage>>,
    consumer_notification_receiver: Option<Receiver<OrderMessage>>,
  ) -> Result<Self> {
    log::debug!("Start RabbitMQ consumer on queue {:?}", queue_name);

    let consumer = channel
      .basic_consume(
        queue_name,
        consumer_tag,
        BasicConsumeOptions::default(),
        FieldTable::default(),
      )
      .await?;

    let mut current_consumer = Some(consumer);

    let channel = Arc::new(channel.clone());
    let queue_name_clone = queue_name.to_string();
    let consumer_tag_clone = consumer_tag.to_string();

    let handle = Some(task::spawn(async move {

      loop {

        // If there is a receiver for messages forwarded by the other consumer...
        if let Some(receiver) = consumer_notification_receiver.clone() {

          // Check whether the other consumer forwarded an order message
          if let Ok(order_from_other_consumer) = receiver.try_recv() {
            log::debug!(
              "Queue {} consumer {} got a message from the other consumer...",
              queue_name_clone,
              consumer_tag_clone
            );

            match order_from_other_consumer {
              OrderMessage::StopConsumingJobs => {
                // Stop consuming jobs queue
                log::info!("Stop consuming {} jobs queue", queue_name_clone);
                if let Err(error) = channel
                  .basic_cancel(&consumer_tag_clone, BasicCancelOptions::default())
                  .await
                {
                  log::error!(
                    "Unable to stop consuming {} jobs queue: {}",
                    queue_name_clone,
                    error.to_string()
                  )
                } else {
                  current_consumer = None;
                }
              }
              OrderMessage::ResumeConsumingJobs => {
                // Resume consuming jobs queue
                log::info!("Resume consuming {} jobs queue", queue_name_clone);
                match channel
                  .basic_consume(
                    &queue_name_clone,
                    &consumer_tag_clone,
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                  )
                  .await
                {
                  Ok(new_consumer) => {
                    current_consumer = Some(new_consumer);
                  }
                  Err(error) => log::error!(
                    "Unable to resume consuming {} jobs queue: {}",
                    queue_name_clone,
                    error.to_string()
                  ),
                }
              }
              _ => log::error!(
                "Unsupported order message from other consumer: {:?}",
                order_from_other_consumer
              ),
            }
          }
        }

        // If there is an AMQP consumer...
        if let Some(mut consumer) = current_consumer.clone() {

          // Check whether a message can be consumed from the AMQP channel
          if let Ok(delivery) = consumer.try_next().await {
            let (_, delivery) = delivery.expect("error in consumer");

            if let Err(error) = Self::process_delivery(
              sender.clone(),
              channel.clone(),
              &delivery,
              &queue_name_clone.clone(),
              current_orders.clone(),
              consumer_notification_sender.clone(),
              consumer_notification_receiver.clone(),
            )
            .await
            {
              log::error!("RabbitMQ consumer: {:?}", error);
              if let Err(error) = publish::error(channel.clone(), &delivery, &error).await {
                log::error!("Unable to publish response: {:?}", error);
              }
            }
          }
        }

        // Wait a bit...
        std::thread::sleep(Duration::from_millis(40));
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
    consumer_notification_sender: Option<Sender<OrderMessage>>,
    consumer_notification_receiver: Option<Receiver<OrderMessage>>,
  ) -> Result<()> {
    let count = helpers::get_message_death_count(&delivery);
    let message_data = std::str::from_utf8(&delivery.data).map_err(|e| {
      MessageError::RuntimeError(format!("unable to retrieve raw message: {:?}", e))
    })?;

    let order_message = OrderMessage::try_from(message_data)?;

    log::debug!(
      "RabbitMQ consumer on {:?} queue received message: {:?} (iteration: {}, delivery: {})",
      queue_name,
      order_message,
      count.unwrap_or(0),
      delivery.delivery_tag,
    );

    match order_message {
      OrderMessage::Job(_) => {
        let (is_initializing, is_starting, has_job) = {
          let current_orders = current_orders.lock().unwrap();

          (
            current_orders.init.is_some(),
            current_orders.start.is_some(),
            current_orders.job.is_some(),
          )
        };

        if is_initializing || is_starting || has_job {
          // Worker already processing
          return Self::reject_delivery(channel, delivery.delivery_tag).await;
        }

        current_orders.lock().unwrap().job = Some(delivery.clone());
      }
      OrderMessage::InitProcess(_) => {
        let is_initializing = current_orders.lock().unwrap().init.is_some();
        let has_job = current_orders.lock().unwrap().job.is_some();

        if is_initializing || has_job {
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

        return Ok(());
      }
      OrderMessage::Status => {
        if current_orders.lock().unwrap().status.is_some() {
          // Worker already checking status
          return Self::reject_delivery(channel, delivery.delivery_tag).await;
        }

        current_orders.lock().unwrap().status = Some(delivery.clone());
      }
      OrderMessage::StopConsumingJobs | OrderMessage::ResumeConsumingJobs => {

        if let Some(sender) = consumer_notification_sender {
          // Forward the received order to the job messages consumer
          sender.send(order_message.clone()).await.map_err(|e| {
            MessageError::RuntimeError(format!(
              "Unable to send {:?} order to other consumer: {:?}",
              order_message, e
            ))
          })?;

          return channel
            .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
            .await
            .map_err(MessageError::Amqp);
        }

        if let Some(_receiver) = consumer_notification_receiver {
          log::warn!(
            "{:?} queue consumer cannot handle correctly such an order message: {:?}",
            queue_name,
            order_message
          );
        } else {
          log::warn!("No order channel set between RabbitMQ consumers");
        }

        return Self::reject_delivery(channel, delivery.delivery_tag).await;
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
    log::warn!("Reject delivery {}", delivery_tag);
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
