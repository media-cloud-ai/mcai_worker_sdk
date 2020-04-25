use lapin::{options::QueueBindOptions, types::FieldTable, Channel};
use std::collections::HashMap;

pub struct BindDescription {
  pub exchange: String,
  pub queue: String,
  pub routing_key: String,
  pub headers: HashMap<String, String>,
}

impl BindDescription {
  pub fn declare(&self, channel: &Channel) {
    if let Err(msg) = channel
      .queue_bind(
        &self.queue.to_string(),
        &self.exchange.to_string(),
        &self.routing_key.to_string(),
        QueueBindOptions::default(),
        FieldTable::default(),
      )
      .wait()
    {
      error!(
        "Unable to bind queue {} to exchange {}: {:?}",
        self.queue, self.exchange, msg
      );
    }
  }
}
