use lapin::options::{BasicAckOptions, BasicPublishOptions, BasicRejectOptions};
use lapin::{BasicProperties, Channel, Promise};

use crate::message_exchange::rabbitmq::{EXCHANGE_NAME_RESPONSE, ROUTING_KEY_JOB_STATUS};
use crate::processor::ProcessStatus;
use lapin::message::Delivery;
use std::sync::Arc;

pub fn job_status(
  channel: Arc<Channel>,
  delivery: &Delivery,
  process_status: ProcessStatus,
) -> Promise<()> {
  let msg = json!(process_status).to_string();

  let result = channel
    .basic_publish(
      EXCHANGE_NAME_RESPONSE,
      ROUTING_KEY_JOB_STATUS,
      BasicPublishOptions::default(),
      msg.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok();

  if result {
    channel.basic_ack(
      delivery.delivery_tag,
      BasicAckOptions::default(), /*not requeue*/
    )
  } else {
    channel.basic_reject(
      delivery.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
  }
}
