use lapin::{message::Delivery, options::BasicRejectOptions, Channel, Promise};
use std::sync::Arc;

pub fn job_missing_requirements(
  channel: Arc<Channel>,
  delivery: &Delivery,
  details: &str,
) -> Promise<()> {
  log::debug!("{}", details);
  channel.basic_reject(delivery.delivery_tag, BasicRejectOptions::default())
}
