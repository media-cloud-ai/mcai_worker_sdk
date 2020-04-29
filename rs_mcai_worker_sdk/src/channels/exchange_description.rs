use amq_protocol_types::AMQPValue;
use lapin::{options::ExchangeDeclareOptions, types::FieldTable, Channel, ExchangeKind};

pub struct ExchangeDescription {
  pub name: String,
  pub kind: ExchangeKind,
  pub alternate_exchange: Option<String>,
}

impl ExchangeDescription {
  pub fn declare(&self, channel: &Channel) {
    let mut exchange_options = ExchangeDeclareOptions::default();
    exchange_options.durable = true;

    let mut field_table = FieldTable::default();
    if let Some(alternate_exchange) = &self.alternate_exchange {
      field_table.insert(
        "alternate-exchange".into(),
        AMQPValue::LongString(alternate_exchange.to_string().into()),
      );
    }

    if let Err(msg) = channel
      .exchange_declare(
        &self.name.to_string(),
        self.kind.clone(),
        exchange_options,
        field_table,
      )
      .wait()
    {
      error!("Unable to create exchange {}: {:?}", self.name, msg);
    }
  }
}
