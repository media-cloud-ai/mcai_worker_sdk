use super::{publish, CurrentOrders};
use crate::{
  message_exchange::{
    rabbitmq::{publish::publish_worker_response, QUEUE_WORKER_CREATED},
    Feedback, ResponseMessage,
  },
  MessageError, Result,
};
use async_std::{
  channel::{self, Receiver, Sender},
  sync::{Arc, Mutex as AsyncMutex},
  task::{self, JoinHandle},
};
use lapin::message::Delivery;
use lapin::Channel;
use std::sync::Mutex;

pub struct RabbitmqPublisher {
  handle: Option<JoinHandle<()>>,
  response_sender: Sender<ResponseMessage>,
}

impl RabbitmqPublisher {
  pub async fn new(channel: &Channel, current_orders: Arc<Mutex<CurrentOrders>>) -> Result<Self> {
    let (response_sender, response_receiver) = channel::unbounded();

    let response_receiver = Arc::new(AsyncMutex::new(response_receiver));

    let channel = Arc::new(channel.clone());

    let handle = Some(task::spawn(async move {
      loop {
        if let Err(error) = Self::handle_response(
          response_receiver.clone(),
          channel.clone(),
          current_orders.clone(),
        )
        .await
        {
          log::error!("{:?}", error);
        }
      }
    }));

    Ok(RabbitmqPublisher {
      handle,
      response_sender,
    })
  }

  pub async fn send_response(&self, response: ResponseMessage) {
    self.response_sender.send(response).await.unwrap();
  }

  async fn handle_response(
    response_receiver: Arc<AsyncMutex<Receiver<ResponseMessage>>>,
    channel: Arc<Channel>,
    current_orders: Arc<Mutex<CurrentOrders>>,
  ) -> Result<()> {
    let response = response_receiver.lock().await.recv().await.map_err(|e| {
      MessageError::RuntimeError(format!(
        "unable to wait response from processor: {:?}",
        e.to_string()
      ))
    })?;

    log::debug!("Response: {:?}", response);
    log::debug!("{}", current_orders.lock().unwrap());

    let deliveries: Vec<Delivery> = match response {
      ResponseMessage::Feedback(Feedback::Progression(progression)) => {
        return publish::job_progression(channel, progression);
      }
      ResponseMessage::WorkerCreated(worker_configuration) => {
        let payload = json!(worker_configuration).to_string();
        return publish_worker_response(channel, None, QUEUE_WORKER_CREATED, &payload).await;
      }
      ResponseMessage::WorkerInitialized(_)
      | ResponseMessage::WorkerStarted(_)
      | ResponseMessage::Completed(_)
      | ResponseMessage::Error(_) => current_orders.lock().unwrap().get_process_deliveries(),
      ResponseMessage::Feedback(_) | ResponseMessage::StatusError(_) => {
        current_orders.lock().unwrap().get_status_deliveries()
      }
    };

    let job_delivery = current_orders.lock().unwrap().get_job_delivery();

    if deliveries.is_empty() {
      match response {
        ResponseMessage::Completed(_) | ResponseMessage::Error(_) => {
          if let Some(job_delivery) = job_delivery {
            if let Err(error) = publish::response_with_delivery(channel.clone(), Some(job_delivery.clone()), &response).await {
              log::error!("Unable to publish response: {:?}", error);
            }
          }
        }
        _ => {
          if let Err(error) = publish::response_with_delivery(channel.clone(), None, &response).await {
            log::error!("Unable to publish response: {:?}", error);
          }
        }
        // if let Some(job_delivery) = job_delivery {
        //   if let Err(error) = publish::response_with_delivery(channel.clone(), Some(job_delivery.clone()), &response).await {
        //     log::error!("Unable to publish response: {:?}", error);
        //   }
        // } else {
        //   if let Err(error) = publish::response_with_delivery(channel.clone(), None, &response).await {
        //     log::error!("Unable to publish response: {:?}", error);
        //   }
        // }
      }
    }

    for delivery in deliveries {
      if let Err(error) =
        publish::response_with_delivery(channel.clone(), Some(delivery.clone()), &response).await
      {
        if let Err(error) = publish::error(channel.clone(), &delivery, &error).await {
          log::error!("Unable to publish response: {:?}", error);
        }
      }
    }

    match response {
      ResponseMessage::WorkerCreated(_)
      | ResponseMessage::WorkerInitialized(_)
      | ResponseMessage::WorkerStarted(_) => {}
      ResponseMessage::Completed(_) | ResponseMessage::Error(_) => {
        current_orders.lock().unwrap().reset_process_deliveries();
      }
      ResponseMessage::Feedback(_) | ResponseMessage::StatusError(_) => {
        current_orders.lock().unwrap().reset_status_deliveries();
      }
    };

    Ok(())
  }
}

impl Drop for RabbitmqPublisher {
  fn drop(&mut self) {
    self.handle.take().map(JoinHandle::cancel);
  }
}
