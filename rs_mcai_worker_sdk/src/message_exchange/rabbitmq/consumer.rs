use super::{helpers, publish};
use crate::{
  job::Job,
  message_exchange::{OrderMessage, ResponseMessage},
  MessageError, Result,
};
use amq_protocol_types::FieldTable;
use async_std::stream::StreamExt;
use async_std::{
  channel::{self, Receiver, Sender},
  sync::{Arc, Mutex},
  task::{self, JoinHandle},
};
use lapin::{message::Delivery, options::BasicConsumeOptions, Channel};

pub struct RabbitmqConsumer {
  handle: Option<JoinHandle<()>>,
  response_sender: Sender<ResponseMessage>,
}

impl RabbitmqConsumer {
  pub async fn new(
    channel: &Channel,
    sender: Sender<OrderMessage>,
    queue_name: &str,
    consumer_tag: &str,
  ) -> Result<Self> {
    let mut consumer = channel
      .basic_consume(
        queue_name,
        consumer_tag,
        BasicConsumeOptions::default(),
        FieldTable::default(),
      )
      .await?;

    let (response_sender, response_receiver) = channel::unbounded();

    let response_receiver = Arc::new(Mutex::new(response_receiver));

    let channel = Arc::new(channel.clone());

    let cloned_response_sender = response_sender.clone();

    let handle = Some(task::spawn(async move {
      while let Some(delivery) = consumer.next().await {
        let (_, delivery) = delivery.expect("error in consumer");

        if let Err(error) = Self::process_delivery(
          sender.clone(),
          channel.clone(),
          response_receiver.clone(),
          &delivery,
        )
        .await {
          log::error!("{:?}", error);
          if let Err(error) = publish::error(channel.clone(), &delivery, &error)
            .await {
              log::error!("Unable to publish response: {:?}", error);
            }
        }
      }
    }));

    Ok(RabbitmqConsumer {
      handle,
      response_sender,
    })
  }

  pub async fn send_response(&self, response: ResponseMessage) {
    self.response_sender.send(response).await.unwrap();
  }

  async fn process_delivery(
    sender: Sender<OrderMessage>,
    channel: Arc<Channel>,
    receiver: Arc<Mutex<Receiver<ResponseMessage>>>,
    delivery: &Delivery,
  ) -> Result<()> {
    let count = helpers::get_message_death_count(&delivery);
    let message_data = std::str::from_utf8(&delivery.data).map_err(|e| {
      MessageError::RuntimeError(format!("unable to retrieve raw message: {:?}", e))
    })?;

    let job = Job::new(message_data)?;

    log::debug!(target: &job.job_id.to_string(),
      "received message: {:?} (iteration: {})",
      job,
      count.unwrap_or(0));

    sender
      .send(OrderMessage::InitProcess(job.clone()))
      .await
      .unwrap();
    sender.send(OrderMessage::StartProcess(job)).await.unwrap();

    loop {
      let response = receiver.lock().await.recv().await.map_err(|e| {
        MessageError::RuntimeError(format!("unable to wait response from processor: {:?}", e))
      })?;

      log::debug!("Response: {:?}", response);
      publish::response(channel.clone(), delivery, &response)
        .await
        .unwrap();
      match response {
        ResponseMessage::Completed(_) | ResponseMessage::Error(_) => {
          return Ok(());
        }
        _ => {}
      }
    }
  }
}

impl Drop for RabbitmqConsumer {
  fn drop(&mut self) {
    self.handle.take().map(JoinHandle::cancel);
  }
}
