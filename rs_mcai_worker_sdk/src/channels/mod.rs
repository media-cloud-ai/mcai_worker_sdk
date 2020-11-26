mod bind_description;
mod exchange_description;
mod queue_description;

use crate::worker::WorkerConfiguration;
pub use bind_description::BindDescription;
pub use exchange_description::ExchangeDescription;
use lapin::{
  options::{BasicPublishOptions, BasicQosOptions, ExchangeDeclareOptions},
  BasicProperties, Channel, Connection, ExchangeKind,
};
pub use queue_description::QueueDescription;
use std::collections::HashMap;

static EXCHANGE_NAME_SUBMIT: &str = "job_submit";
static EXCHANGE_NAME_RESPONSE: &str = "job_response";
static EXCHANGE_NAME_DELAYED: &str = "job_delayed";
static EXCHANGE_NAME_DIRECT_MESSAGING: &str = "direct_messaging";
pub(crate) static EXCHANGE_NAME_DIRECT_MESSAGING_RESPONSE: &str = "direct_messaging_response";
static EXCHANGE_NAME_DIRECT_MESSAGING_NOT_FOUND: &str = "direct_messaging_not_found";
static EXCHANGE_NAME_RESPONSE_DELAYED: &str = "job_response_delayed";

static QUEUE_NAME_WORKER_DISCOVERY: &str = "worker_discovery";

pub fn declare_consumer_channel(
  conn: &Connection,
  worker_configuration: &WorkerConfiguration,
) -> Channel {
  let channel = conn.create_channel().wait().unwrap();
  let prefetch_count = 1;

  info!("Initialise Exchanges and Queues");
  set_qos(&channel, prefetch_count);

  let mut exchange_options = ExchangeDeclareOptions::default();
  exchange_options.durable = true;

  let delayed_exchange = ExchangeDescription {
    name: EXCHANGE_NAME_DELAYED.to_string(),
    kind: ExchangeKind::Fanout,
    alternate_exchange: None,
  };
  delayed_exchange.declare(&channel);

  let submit_exchange = ExchangeDescription {
    name: EXCHANGE_NAME_SUBMIT.to_string(),
    kind: ExchangeKind::Topic,
    alternate_exchange: Some("job_queue_not_found".to_string()),
  };
  submit_exchange.declare(&channel);

  let response_exchange = ExchangeDescription {
    name: EXCHANGE_NAME_RESPONSE.to_string(),
    kind: ExchangeKind::Topic,
    alternate_exchange: Some("job_response_not_found".to_string()),
  };
  response_exchange.declare(&channel);

  let delayed_queue = QueueDescription {
    name: EXCHANGE_NAME_DELAYED.to_string(),
    durable: true,
    auto_delete: false,
    dead_letter_exchange: Some("".to_string()),
    dead_letter_routing_key: None,
    max_priority: None,
    message_ttl: Some(5000),
  };
  delayed_queue.declare(&channel);

  let delayed_bind = BindDescription {
    exchange: EXCHANGE_NAME_DELAYED.to_string(),
    queue: EXCHANGE_NAME_DELAYED.to_string(),
    routing_key: "*".to_string(),
    headers: HashMap::new(),
  };
  delayed_bind.declare(&channel);

  let direct_messaging_exchange = ExchangeDescription {
    name: EXCHANGE_NAME_DIRECT_MESSAGING.to_string(),
    kind: ExchangeKind::Headers,
    alternate_exchange: Some("direct_messaging_not_found".to_string()),
  };
  direct_messaging_exchange.declare(&channel);

  let direct_messaging_queue = QueueDescription {
    name: worker_configuration.get_direct_messaging_queue_name(),
    durable: false,
    auto_delete: true,
    dead_letter_exchange: None,
    dead_letter_routing_key: None,
    max_priority: None,
    message_ttl: None,
  };
  direct_messaging_queue.declare(&channel);

  let direct_message_response_exchange = ExchangeDescription {
    name: EXCHANGE_NAME_DIRECT_MESSAGING_RESPONSE.to_string(),
    kind: ExchangeKind::Topic,
    alternate_exchange: Some("direct_messaging_not_found".to_string()),
  };
  direct_message_response_exchange.declare(&channel);

  let direct_messaging_not_found_queue = QueueDescription {
    name: EXCHANGE_NAME_DIRECT_MESSAGING_NOT_FOUND.to_string(),
    durable: true,
    auto_delete: false,
    dead_letter_exchange: None,
    dead_letter_routing_key: None,
    max_priority: None,
    message_ttl: None,
  };
  direct_messaging_not_found_queue.declare(&channel);

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

  let direct_message_bind = BindDescription {
    exchange: EXCHANGE_NAME_DIRECT_MESSAGING.to_string(),
    queue: worker_configuration.get_direct_messaging_queue_name(),
    routing_key: "*".to_string(),
    headers: direct_messaging_exchange_headers,
  };
  direct_message_bind.declare(&channel);

  let worker_discovery_queue = QueueDescription {
    name: QUEUE_NAME_WORKER_DISCOVERY.to_string(),
    durable: true,
    auto_delete: false,
    dead_letter_exchange: Some(EXCHANGE_NAME_RESPONSE_DELAYED.to_string()),
    dead_letter_routing_key: Some(QUEUE_NAME_WORKER_DISCOVERY.to_string()),
    max_priority: None,
    message_ttl: None,
  };
  worker_discovery_queue.declare(&channel);

  let payload = json!(worker_configuration).to_string();

  if let Err(msg) = channel
    .basic_publish(
      "",
      QUEUE_NAME_WORKER_DISCOVERY,
      BasicPublishOptions::default(),
      payload.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
  {
    error!(
      "Impossible to send message on {} queue: {:?}",
      QUEUE_NAME_WORKER_DISCOVERY, msg
    );
  }

  let job_queue = QueueDescription {
    name: worker_configuration.get_queue_name(),
    durable: true,
    auto_delete: false,
    dead_letter_exchange: Some(EXCHANGE_NAME_DELAYED.to_string()),
    dead_letter_routing_key: Some(worker_configuration.get_queue_name()),
    max_priority: Some(100),
    message_ttl: None,
  };
  job_queue.declare(&channel);

  let delayed_bind = BindDescription {
    exchange: EXCHANGE_NAME_SUBMIT.to_string(),
    queue: worker_configuration.get_queue_name(),
    routing_key: worker_configuration.get_queue_name(),
    headers: HashMap::new(),
  };
  delayed_bind.declare(&channel);

  info!("Exchanges and Queues are configured.");
  channel
}

fn set_qos(channel: &Channel, prefetch_count: u16) {
  if let Err(msg) = channel
    .basic_qos(prefetch_count, BasicQosOptions::default())
    .wait()
  {
    error!("Unable to set QoS on channels: {:?}", msg);
  }
}
