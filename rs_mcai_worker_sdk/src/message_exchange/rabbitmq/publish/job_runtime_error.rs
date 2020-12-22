use crate::message_exchange::rabbitmq::{EXCHANGE_NAME_RESPONSE, QUEUE_JOB_ERROR};
use lapin::{
  message::Delivery,
  options::{BasicAckOptions, BasicPublishOptions, BasicRejectOptions},
  BasicProperties, Channel, Promise,
};
use std::sync::Arc;

pub fn job_runtime_error(channel: Arc<Channel>, message: &Delivery, details: &str) -> Promise<()> {
  log::error!("An error occurred: {:?}", details);
  let content = json!({
    "status": "error",
    "message": details
  })
  .to_string();

  if channel
    .basic_publish(
      EXCHANGE_NAME_RESPONSE,
      QUEUE_JOB_ERROR,
      BasicPublishOptions::default(),
      content.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok()
  {
    channel.basic_ack(
      message.delivery_tag,
      BasicAckOptions::default(), /*not requeue*/
    )
  } else {
    channel.basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
  }
}
