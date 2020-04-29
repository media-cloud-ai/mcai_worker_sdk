use amq_protocol_types::AMQPValue;
use lapin::{options::QueueDeclareOptions, types::FieldTable, Channel};

pub struct QueueDescription {
  pub name: String,
  pub durable: bool,
  pub auto_delete: bool,
  pub dead_letter_exchange: Option<String>,
  pub dead_letter_routing_key: Option<String>,
  pub max_priority: Option<i16>,
  pub message_ttl: Option<i16>,
}

impl QueueDescription {
  pub fn declare(&self, channel: &Channel) {
    let declare_options = QueueDeclareOptions {
      durable: self.durable,
      auto_delete: self.auto_delete,
      ..Default::default()
    };

    let mut queue_fields = FieldTable::default();
    if let Some(dead_letter_exchange) = &self.dead_letter_exchange {
      queue_fields.insert(
        "x-dead-letter-exchange".into(),
        AMQPValue::LongString(dead_letter_exchange.to_string().into()),
      );
    }

    if let Some(dead_letter_routing_key) = &self.dead_letter_routing_key {
      queue_fields.insert(
        "x-dead-letter-routing-key".into(),
        AMQPValue::LongString(dead_letter_routing_key.to_string().into()),
      );
    }

    if let Some(max_priority) = &self.max_priority {
      queue_fields.insert("x-max-priority".into(), AMQPValue::ShortInt(*max_priority));
    }

    if let Some(message_ttl) = &self.message_ttl {
      queue_fields.insert("x-message-ttl".into(), AMQPValue::ShortInt(*message_ttl));
    }

    if let Err(msg) = channel
      .queue_declare(&self.name.to_string(), declare_options, queue_fields)
      .wait()
    {
      error!("Unable to create queue {}: {:?}", self.name, msg);
    }
  }
}
