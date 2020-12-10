use lapin::{message::Delivery, options::BasicRejectOptions, Channel, Promise};
use std::sync::Arc;

pub fn job_not_implemented(channel: Arc<Channel>, delivery: &Delivery) -> Promise<()> {
  log::error!("Not implemented feature");
  channel.basic_reject(
    delivery.delivery_tag,
    BasicRejectOptions { requeue: true }, /*requeue*/
  )
}
