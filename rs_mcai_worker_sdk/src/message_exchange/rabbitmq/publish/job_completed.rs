use lapin::message::Delivery;
use lapin::options::{BasicAckOptions, BasicPublishOptions, BasicRejectOptions};
use lapin::{BasicProperties, Promise};

use crate::message::QUEUE_JOB_COMPLETED;
use crate::message::RESPONSE_EXCHANGE;
use crate::JobResult;
use crate::McaiChannel;

pub fn job_completed(
  channel: McaiChannel,
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
