use lapin::message::Delivery;
use lapin::options::{BasicAckOptions, BasicPublishOptions, BasicRejectOptions};
use lapin::{BasicProperties, Channel, Promise};

use crate::{
  message_exchange::rabbitmq::{QUEUE_JOB_COMPLETED, RESPONSE_EXCHANGE},
  JobResult,
};
use std::sync::Arc;

pub fn job_completed(
  channel: Arc<Channel>,
  delivery: &Delivery,
  job_result: &JobResult,
) -> Promise<()> {
  let msg = json!(job_result).to_string();

  let result = channel
    .basic_publish(
      RESPONSE_EXCHANGE,
      QUEUE_JOB_COMPLETED,
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
