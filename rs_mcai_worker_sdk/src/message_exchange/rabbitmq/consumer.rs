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
  options::{BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicRejectOptions},
  Channel,
};
use std::{
  convert::TryFrom,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
  },
};

pub struct RabbitmqConsumer {
  handle: Option<JoinHandle<()>>,
  pub should_consume: Arc<AtomicBool>,
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
    job_consumer_triggers: Vec<Arc<AtomicBool>>,
  ) -> Result<Self> {
    log::debug!("Start RabbitMQ consumer on queue {:?}", queue_name);

    let mut optional_consumer = Some(
      channel
        .basic_consume(
          queue_name,
          consumer_tag,
          BasicConsumeOptions::default(),
          FieldTable::default(),
        )
        .await?,
    );

    let channel = Arc::new(channel.clone());
    let queue_name_clone = queue_name.to_string();
    let consumer_tag_clone = consumer_tag.to_string();

    let should_consume = Arc::new(AtomicBool::new(true));
    let should_consume_clone = should_consume.clone();

    let job_consumer_triggers_clone = job_consumer_triggers.clone();

    let handle = Some(task::spawn(async move {
      loop {
        if optional_consumer.is_some() && !should_consume_clone.load(Ordering::Relaxed) {
          // if should not consume, unregister consumer from channel
          optional_consumer = None;

          channel
            .basic_cancel(&consumer_tag_clone, BasicCancelOptions::default())
            .await
            .map_err(MessageError::Amqp)
            .unwrap();
        } else if optional_consumer.is_none() && should_consume_clone.load(Ordering::Relaxed) {
          // if should consume, reset channel consumer
          optional_consumer = Some(
            channel
              .basic_consume(
                &queue_name_clone,
                &consumer_tag_clone,
                BasicConsumeOptions::default(),
                FieldTable::default(),
              )
              .await
              .unwrap(),
          );
        }

        if let Some(mut consumer) = optional_consumer.clone() {
          // Consume messages
          if let Some(delivery) = consumer.next().await {
            let (_, delivery) = delivery.expect("error in consumer");

            if !should_consume_clone.load(Ordering::Relaxed) {
              // if should have not consumed a message, reject it and unregister from channel

              optional_consumer = None;

              Self::reject_delivery(channel.clone(), delivery.delivery_tag)
                .await
                .unwrap();

              channel
                .basic_cancel(&consumer_tag_clone, BasicCancelOptions::default())
                .await
                .map_err(MessageError::Amqp)
                .unwrap();
            } else {
              // else process received message
              if let Err(error) = Self::process_delivery(
                sender.clone(),
                channel.clone(),
                &delivery,
                &queue_name_clone.clone(),
                current_orders.clone(),
                job_consumer_triggers_clone.clone(),
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
        }
      }
    }));

    Ok(RabbitmqConsumer {
      handle,
      should_consume,
    })
  }

  async fn process_delivery(
    sender: Sender<OrderMessage>,
    channel: Arc<Channel>,
    delivery: &Delivery,
    queue_name: &str,
    current_orders: Arc<Mutex<CurrentOrders>>,
    job_consumer_triggers: Vec<Arc<AtomicBool>>,
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
      OrderMessage::StopConsumingJobs => {
        // Stop consuming jobs queues
        for trigger in job_consumer_triggers {
          trigger.store(false, Ordering::Relaxed);
        }

        return channel
          .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
          .await
          .map_err(MessageError::Amqp);
      }
      OrderMessage::ResumeConsumingJobs => {
        // Resume consuming jobs queues
        for trigger in job_consumer_triggers {
          trigger.store(true, Ordering::Relaxed);
        }

        return channel
          .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
          .await
          .map_err(MessageError::Amqp);
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
