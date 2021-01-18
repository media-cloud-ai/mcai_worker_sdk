use amq_protocol_types::AMQPValue;
use lapin::{options::QueueDeclareOptions, types::FieldTable, Channel};

#[derive(Default)]
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

    let queue_fields = self.get_field_table();

    if let Err(msg) = channel
      .queue_declare(&self.name.to_string(), declare_options, queue_fields)
      .wait()
    {
      log::error!("Unable to create queue {}: {:?}", self.name, msg);
    }
  }

  fn get_field_table(&self) -> FieldTable {
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
    queue_fields
  }
}

#[test]
pub fn test_queue_description() {
  let name = "queue_name".to_string();
  let durable = true;
  let auto_delete = false;
  let dead_letter_exchange = Some("dead_letter_exchange_key".to_string());
  let dead_letter_routing_key = Some("dead_letter_routing_key".to_string());
  let max_priority = Some(1000);
  let message_ttl = Some(123);

  let queue_description = QueueDescription {
    name,
    durable,
    auto_delete,
    dead_letter_exchange: dead_letter_exchange.clone(),
    dead_letter_routing_key: dead_letter_routing_key.clone(),
    max_priority,
    message_ttl,
  };

  let field_table = queue_description.get_field_table();
  let tree_map = field_table.inner();
  assert!(tree_map.contains_key("x-dead-letter-exchange"));
  assert!(tree_map.contains_key("x-dead-letter-routing-key"));
  assert!(tree_map.contains_key("x-max-priority"));
  assert!(tree_map.contains_key("x-message-ttl"));

  assert_eq!(
    &AMQPValue::LongString(dead_letter_exchange.unwrap().into()),
    tree_map.get("x-dead-letter-exchange").unwrap()
  );
  assert_eq!(
    &AMQPValue::LongString(dead_letter_routing_key.unwrap().into()),
    tree_map.get("x-dead-letter-routing-key").unwrap()
  );
  assert_eq!(
    &AMQPValue::ShortInt(max_priority.unwrap()),
    tree_map.get("x-max-priority").unwrap()
  );
  assert_eq!(
    &AMQPValue::ShortInt(message_ttl.unwrap()),
    tree_map.get("x-message-ttl").unwrap()
  );
}
