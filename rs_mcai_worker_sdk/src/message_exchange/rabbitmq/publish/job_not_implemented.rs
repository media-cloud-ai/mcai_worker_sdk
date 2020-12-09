use crate::McaiChannel;
use lapin::{message::Delivery, options::BasicRejectOptions, Promise};

pub fn job_not_implemented(channel: McaiChannel, delivery: &Delivery) -> Promise<()> {
  log::error!("Not implemented feature");
  channel.basic_reject(
    delivery.delivery_tag,
    BasicRejectOptions { requeue: true }, /*requeue*/
  )
}
