use amq_protocol_types::{AMQPValue, LongString, ShortString};
use lapin::{options::QueueBindOptions, Channel};
use std::collections::{BTreeMap, HashMap};

pub struct BindDescription {
  pub exchange: String,
  pub queue: String,
  pub routing_key: String,
  pub headers: HashMap<String, String>,
}

impl BindDescription {
  pub fn declare(&self, channel: &Channel) {
    let mut headers = BTreeMap::new();

    for (key, value) in &self.headers {
      let key = ShortString::from(key.clone());
      let value = AMQPValue::LongString(LongString::from(value.clone()));

      headers.insert(key, value);
    }

    if let Err(msg) = channel
      .queue_bind(
        &self.queue.to_string(),
        &self.exchange.to_string(),
        &self.routing_key.to_string(),
        QueueBindOptions::default(),
        headers.into(),
      )
      .wait()
    {
      log::error!(
        "Unable to bind queue {} to exchange {}: {:?}",
        self.queue,
        self.exchange,
        msg
      );
    }
  }
}

#[test]
pub fn test_queue_description() {
  let exchange = "exchange_name".to_string();
  let queue = "queue_name".to_string();
  let routing_key = "routing_key".to_string();
  let headers = HashMap::<String, String>::new();

  let bind_description = BindDescription {
    exchange: exchange.clone(),
    queue: queue.clone(),
    routing_key: routing_key.clone(),
    headers: headers.clone(),
  };

  assert_eq!(exchange, bind_description.exchange);
  assert_eq!(queue, bind_description.queue);
  assert_eq!(routing_key, bind_description.routing_key);
  assert_eq!(headers, bind_description.headers);
}
