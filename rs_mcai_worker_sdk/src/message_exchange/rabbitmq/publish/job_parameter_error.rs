use crate::McaiChannel;
use lapin::{message::Delivery, options::BasicRejectOptions, Promise};

pub fn job_parameter_error(channel: McaiChannel, message: &Delivery, details: &str) -> Promise<()> {
  log::debug!("Parameter value error: {}", details);
  channel.basic_reject(message.delivery_tag, BasicRejectOptions::default())
}
