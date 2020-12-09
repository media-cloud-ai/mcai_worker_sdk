use crate::McaiChannel;
use lapin::{message::Delivery, options::BasicRejectOptions, Promise};

pub fn job_missing_requirements(
  channel: McaiChannel,
  delivery: &Delivery,
  details: &str,
) -> Promise<()> {
  log::debug!("{}", details);
  channel.basic_reject(delivery.delivery_tag, BasicRejectOptions::default())
}
