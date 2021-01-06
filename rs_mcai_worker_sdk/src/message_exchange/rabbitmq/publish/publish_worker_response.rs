use crate::{message_exchange::rabbitmq::EXCHANGE_NAME_WORKER_RESPONSE, Result};
use lapin::{
  message::Delivery,
  options::{BasicAckOptions, BasicPublishOptions, BasicRejectOptions},
  BasicProperties, Channel,
};
use std::sync::Arc;

pub async fn publish_worker_response(
  channel: Arc<Channel>,
  delivery: &Delivery,
  queue_name: &str,
  payload: &str,
) -> Result<()> {
  let result = channel
    .basic_publish(
      EXCHANGE_NAME_WORKER_RESPONSE,
      queue_name,
      BasicPublishOptions::default(),
      payload.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok();

  if result {
    channel
      .basic_ack(
        delivery.delivery_tag,
        BasicAckOptions::default(), /*not requeue*/
      )
      .await
      .map_err(|e| e.into())
  } else {
    channel
      .basic_reject(
        delivery.delivery_tag,
        BasicRejectOptions { requeue: true }, /*requeue*/
      )
      .await
      .map_err(|e| e.into())
  }
}
