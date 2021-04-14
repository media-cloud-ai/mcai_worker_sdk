use super::{helpers, publish, CurrentOrders};
use crate::{message_exchange::OrderMessage, MessageError, Result};
use amq_protocol_types::FieldTable;
use async_std::{
  channel::Sender,
  future::timeout,
  stream::StreamExt,
  task::{self, JoinHandle},
};
use lapin::{
  message::Delivery,
  options::{
    BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicNackOptions, BasicRejectOptions,
  },
  Channel,
};
use std::{
  convert::TryFrom,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
  },
  time::Duration,
};

pub struct RabbitmqConsumer {
  handle: Option<JoinHandle<()>>,
  should_consume: Arc<AtomicBool>,
  consumer_triggers: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
}

pub const RABBITMQ_CONSUMER_TAG_JOB: &str = "amqp_worker";
pub const RABBITMQ_CONSUMER_TAG_DIRECT: &str = "status_amqp_worker";

const CONSUMER_TIMEOUT: Duration = Duration::from_millis(500);

impl RabbitmqConsumer {
  pub async fn new(
    channel: &Channel,
    sender: Sender<OrderMessage>,
    queue_name: &str,
    consumer_tag: &str,
    current_orders: Arc<Mutex<CurrentOrders>>,
  ) -> Result<Self> {
    log::debug!("Start RabbitMQ consumer on queue {:?}", queue_name);

    let channel = Arc::new(channel.clone());

    let should_consume = Arc::new(AtomicBool::new(true));
    let consumer_triggers = Arc::new(Mutex::new(vec![]));

    let handle = Self::start_consumer(
      channel,
      queue_name,
      consumer_tag,
      sender,
      current_orders,
      should_consume.clone(),
      consumer_triggers.clone(),
    )
    .await?;

    Ok(RabbitmqConsumer {
      handle,
      should_consume,
      consumer_triggers,
    })
  }

  pub fn connect(&mut self, rabbitmq_consumer: &RabbitmqConsumer) {
    self
      .consumer_triggers
      .lock()
      .unwrap()
      .push(rabbitmq_consumer.get_trigger());
  }

  fn get_trigger(&self) -> Arc<AtomicBool> {
    self.should_consume.clone()
  }

  async fn start_consumer(
    channel: Arc<Channel>,
    queue_name: &str,
    consumer_tag: &str,
    sender: Sender<OrderMessage>,
    current_orders: Arc<Mutex<CurrentOrders>>,
    should_consume: Arc<AtomicBool>,
    consumer_triggers: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
  ) -> Result<Option<JoinHandle<()>>> {
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

    let queue_name = queue_name.to_string();
    let consumer_tag = consumer_tag.to_string();

    let handle = task::spawn(async move {
      loop {
        match (&optional_consumer, should_consume.load(Ordering::Relaxed)) {
          (Some(_), false) => {
            // if should not consume, unregister consumer from channel
            log::debug!("{} consumer unregisters from channel...", queue_name);

            optional_consumer = None;

            channel
              .basic_cancel(&consumer_tag, BasicCancelOptions::default())
              .await
              .map_err(MessageError::Amqp)
              .unwrap();
          }
          (None, true) => {
            // if should consume, reset channel consumer
            log::debug!("{} consumer resume consuming channel...", queue_name);

            optional_consumer = Some(
              channel
                .basic_consume(
                  &queue_name,
                  &consumer_tag,
                  BasicConsumeOptions::default(),
                  FieldTable::default(),
                )
                .await
                .unwrap(),
            );
          }
          _ => {}
        }

        if let Some(mut consumer) = optional_consumer.clone() {
          // Consume messages with timeout
          let next_delivery = timeout(CONSUMER_TIMEOUT, consumer.next()).await;

          if let Ok(Some(delivery)) = next_delivery {
            let (_, delivery) = delivery.expect("error in consumer");

            if !should_consume.load(Ordering::Relaxed) {
              // if should have not consumed a message, reject it and unregister from channel

              log::warn!(
                "{} consumer nacks and requeues received message, and unregisters from channel...",
                queue_name
              );

              optional_consumer = None;

              channel
                .basic_nack(
                  delivery.delivery_tag,
                  BasicNackOptions {
                    requeue: true,
                    ..Default::default()
                  },
                )
                .await
                .unwrap();

              channel
                .basic_cancel(&consumer_tag, BasicCancelOptions::default())
                .await
                .map_err(MessageError::Amqp)
                .unwrap();
            } else {
              // else process received message
              if let Err(error) = Self::process_delivery(
                sender.clone(),
                channel.clone(),
                &delivery,
                &queue_name.clone(),
                current_orders.clone(),
                consumer_triggers.clone(),
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
    });

    Ok(Some(handle))
  }

  async fn process_delivery(
    sender: Sender<OrderMessage>,
    channel: Arc<Channel>,
    delivery: &Delivery,
    queue_name: &str,
    current_orders: Arc<Mutex<CurrentOrders>>,
    consumer_triggers: Arc<Mutex<Vec<Arc<AtomicBool>>>>,
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
        for trigger in consumer_triggers.lock().unwrap().iter() {
          trigger.store(false, Ordering::Relaxed);
        }

        return channel
          .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
          .await
          .map_err(MessageError::Amqp);
      }
      OrderMessage::ResumeConsumingJobs => {
        // Resume consuming jobs queues
        for trigger in consumer_triggers.lock().unwrap().iter() {
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
