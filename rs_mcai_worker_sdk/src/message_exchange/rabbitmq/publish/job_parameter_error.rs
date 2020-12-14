use lapin::{message::Delivery, options::BasicRejectOptions, Channel, Promise};
use std::sync::Arc;

pub fn job_parameter_error(
  channel: Arc<Channel>,
  message: &Delivery,
  details: &str,
) -> Promise<()> {
  log::debug!("Parameter value error: {}", details);
  channel.basic_reject(message.delivery_tag, BasicRejectOptions::default())
}
