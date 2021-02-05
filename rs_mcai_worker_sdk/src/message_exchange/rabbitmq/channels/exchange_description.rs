use amq_protocol_types::AMQPValue;
use lapin::{options::ExchangeDeclareOptions, types::FieldTable, Channel, ExchangeKind};

pub struct ExchangeDescription {
  pub name: String,
  pub kind: ExchangeKind,
  pub alternate_exchange: Option<String>,
}

impl ExchangeDescription {
  pub fn new(name: &str, kind: ExchangeKind) -> Self {
    ExchangeDescription {
      name: name.to_owned(),
      kind,
      alternate_exchange: None,
    }
  }

  pub fn with_alternate_exchange(mut self, alternate_exchange: &str) -> Self {
    self.alternate_exchange = Some(alternate_exchange.to_string());
    self
  }

  pub fn declare(&self, channel: &Channel) {
    let exchange_options = ExchangeDeclareOptions {
      durable: true,
      ..Default::default()
    };

    let field_table = self.get_field_table();

    if let Err(msg) = channel
      .exchange_declare(
        &self.name.to_string(),
        self.kind.clone(),
        exchange_options,
        field_table,
      )
      .wait()
    {
      log::error!("Unable to create exchange {}: {:?}", self.name, msg);
    }
  }

  fn get_field_table(&self) -> FieldTable {
    let mut field_table = FieldTable::default();
    if let Some(alternate_exchange) = &self.alternate_exchange {
      field_table.insert(
        "alternate-exchange".into(),
        AMQPValue::LongString(alternate_exchange.to_string().into()),
      );
    }
    field_table
  }
}

#[test]
pub fn test_exchange_description() {
  let name = "exchange_name".to_string();
  let kind = ExchangeKind::Direct;
  let alternate_exchange = Some("alternate_exchange_name".to_string());

  let exchange_description = ExchangeDescription {
    name,
    kind,
    alternate_exchange: alternate_exchange.clone(),
  };

  let field_table = exchange_description.get_field_table();
  let tree_map = field_table.inner();
  assert!(tree_map.contains_key("alternate-exchange"));

  let amqp_value = tree_map.get("alternate-exchange");
  assert!(amqp_value.is_some());

  assert_eq!(
    &AMQPValue::LongString(alternate_exchange.unwrap().into()),
    amqp_value.unwrap()
  );
}
