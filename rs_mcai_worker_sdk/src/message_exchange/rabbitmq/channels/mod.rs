mod bind_description;
mod exchange_description;
mod queue_description;

use crate::message_exchange::rabbitmq::{
  DIRECT_MESSAGING_NOT_FOUND, EXCHANGE_NAME_DELAYED, EXCHANGE_NAME_DIRECT_MESSAGING,
  EXCHANGE_NAME_JOB_RESPONSE, EXCHANGE_NAME_RESPONSE_DELAYED, EXCHANGE_NAME_SUBMIT,
  EXCHANGE_NAME_WORKER_RESPONSE, JOB_QUEUE_NOT_FOUND, JOB_RESPONSE_NOT_FOUND,
  QUEUE_WORKER_DISCOVERY, WORKER_RESPONSE_NOT_FOUND,
};
use crate::worker::WorkerConfiguration;
pub use bind_description::BindDescription;
pub use exchange_description::ExchangeDescription;
use lapin::{
  options::{BasicPublishOptions, BasicQosOptions, ExchangeDeclareOptions},
  BasicProperties, Channel, Connection, ExchangeKind,
};
pub use queue_description::QueueDescription;
use std::collections::HashMap;

pub fn declare_consumer_channel(
  conn: &Connection,
  worker_configuration: &WorkerConfiguration,
) -> Channel {
  let channel = conn.create_channel().wait().unwrap();
  let prefetch_count = 1;

  log::info!("Initialise Exchanges and Queues");
  set_qos(&channel, prefetch_count);

  let mut exchange_options = ExchangeDeclareOptions::default();
  exchange_options.durable = true;

  let delayed_exchange = ExchangeDescription::new(EXCHANGE_NAME_DELAYED, ExchangeKind::Fanout);
  delayed_exchange.declare(&channel);

  ExchangeDescription::new(EXCHANGE_NAME_SUBMIT, ExchangeKind::Topic)
    .with_alternate_exchange(JOB_QUEUE_NOT_FOUND)
    .declare(&channel);

  ExchangeDescription::new(EXCHANGE_NAME_JOB_RESPONSE, ExchangeKind::Topic)
    .with_alternate_exchange(JOB_RESPONSE_NOT_FOUND)
    .declare(&channel);

  ExchangeDescription::new(EXCHANGE_NAME_WORKER_RESPONSE, ExchangeKind::Topic)
    .with_alternate_exchange(WORKER_RESPONSE_NOT_FOUND)
    .declare(&channel);

  let delayed_queue = QueueDescription {
    name: EXCHANGE_NAME_DELAYED.to_string(),
    durable: true,
    dead_letter_exchange: Some("".to_string()),
    message_ttl: Some(5000),
    ..Default::default()
  };
  delayed_queue.declare(&channel);

  let delayed_bind = BindDescription {
    exchange: EXCHANGE_NAME_DELAYED.to_string(),
    queue: EXCHANGE_NAME_DELAYED.to_string(),
    routing_key: "*".to_string(),
    headers: HashMap::new(),
  };
  delayed_bind.declare(&channel);

  let direct_messaging_exchange =
    ExchangeDescription::new(EXCHANGE_NAME_DIRECT_MESSAGING, ExchangeKind::Headers)
      .with_alternate_exchange(DIRECT_MESSAGING_NOT_FOUND);

  direct_messaging_exchange.declare(&channel);

  let direct_messaging_queue = QueueDescription {
    name: worker_configuration.get_direct_messaging_queue_name(),
    durable: false,
    auto_delete: true,
    ..Default::default()
  };
  direct_messaging_queue.declare(&channel);

  let direct_messaging_exchange_headers: HashMap<String, String> = [
    ("broadcast".to_string(), "true".to_string()),
    (
      "instance_id".to_string(),
      worker_configuration.get_instance_id(),
    ),
    (
      "consumer_mode".to_string(),
      worker_configuration.get_consumer_mode(),
    ),
    (
      "job_type".to_string(),
      worker_configuration.get_queue_name(),
    ),
    (
      "worker_name".to_string(),
      worker_configuration.get_worker_name(),
    ),
    (
      "worker_version".to_string(),
      worker_configuration.get_worker_version(),
    ),
    ("x-match".to_string(), "any".to_string()),
  ]
  .iter()
  .cloned()
  .collect();

  let delayed_bind = BindDescription {
    exchange: EXCHANGE_NAME_DIRECT_MESSAGING.to_string(),
    queue: worker_configuration.get_direct_messaging_queue_name(),
    routing_key: "*".to_string(),
    headers: direct_messaging_exchange_headers,
  };
  delayed_bind.declare(&channel);

  let worker_discovery_queue = QueueDescription {
    name: QUEUE_WORKER_DISCOVERY.to_string(),
    durable: true,
    dead_letter_exchange: Some(EXCHANGE_NAME_RESPONSE_DELAYED.to_string()),
    dead_letter_routing_key: Some(QUEUE_WORKER_DISCOVERY.to_string()),
    ..Default::default()
  };
  worker_discovery_queue.declare(&channel);

  let payload = json!(worker_configuration).to_string();

  if let Err(msg) = channel
    .basic_publish(
      EXCHANGE_NAME_JOB_RESPONSE,
      QUEUE_WORKER_DISCOVERY,
      BasicPublishOptions::default(),
      payload.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
  {
    log::error!(
      "Impossible to send message on {} queue: {:?}",
      QUEUE_WORKER_DISCOVERY,
      msg
    );
  }

  let job_queue = QueueDescription {
    name: worker_configuration.get_queue_name(),
    durable: true,
    dead_letter_exchange: Some(EXCHANGE_NAME_DELAYED.to_string()),
    dead_letter_routing_key: Some(worker_configuration.get_queue_name()),
    max_priority: Some(100),
    ..Default::default()
  };
  job_queue.declare(&channel);

  let delayed_bind = BindDescription {
    exchange: EXCHANGE_NAME_SUBMIT.to_string(),
    queue: worker_configuration.get_queue_name(),
    routing_key: worker_configuration.get_queue_name(),
    headers: HashMap::new(),
  };
  delayed_bind.declare(&channel);

  QueueDescription {
    name: WORKER_RESPONSE_NOT_FOUND.to_string(),
    durable: true,
    ..Default::default()
  }
  .declare(&channel);

  log::info!("Exchanges and Queues are configured.");
  channel
}

fn set_qos(channel: &Channel, prefetch_count: u16) {
  if let Err(msg) = channel
    .basic_qos(prefetch_count, BasicQosOptions::default())
    .wait()
  {
    log::error!("Unable to set QoS on channels: {:?}", msg);
  }
}
