use crate::channels::EXCHANGE_NAME_DIRECT_MESSAGING_RESPONSE;
use crate::worker::WorkerConfiguration;
use lapin::{
  message::Delivery,
  options::{BasicAckOptions, BasicPublishOptions, BasicRejectOptions},
  BasicProperties, Channel, Promise,
};
use sysinfo::SystemExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInformation {
  docker_container_id: String,
  total_memory: u64,
  used_memory: u64,
  total_swap: u64,
  used_swap: u64,
  number_of_processors: usize,
}

impl SystemInformation {
  fn new(worker_configuration: &WorkerConfiguration) -> Self {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let docker_container_id = worker_configuration.get_instance_id();
    let total_memory = system.get_total_memory();
    let used_memory = system.get_used_memory();
    let total_swap = system.get_total_swap();
    let used_swap = system.get_used_swap();
    let number_of_processors = system.get_processors().len();

    SystemInformation {
      docker_container_id,
      total_memory,
      used_memory,
      total_swap,
      used_swap,
      number_of_processors,
    }
  }
}

pub fn send_real_time_information(
  message: Delivery,
  channel: &Channel,
  worker_configuration: &WorkerConfiguration,
) -> Promise<()> {
  let information = SystemInformation::new(worker_configuration);
  let serialized = serde_json::to_string(&information).unwrap();

  let result = channel
    .basic_publish(
      EXCHANGE_NAME_DIRECT_MESSAGING_RESPONSE,
      "worker_status_response",
      BasicPublishOptions::default(),
      serialized.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok();

  if result {
    channel.basic_ack(
      message.delivery_tag,
      BasicAckOptions::default(), /*not requeue*/
    )
  } else {
    channel.basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
  }
}
