use crate::worker::WorkerConfiguration;
use sysinfo::SystemExt;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SystemInformation {
  pub docker_container_id: String,
  pub total_memory: u64,
  pub used_memory: u64,
  pub total_swap: u64,
  pub used_swap: u64,
  pub number_of_processors: usize,
}

impl SystemInformation {
  pub(crate) fn new(worker_configuration: &WorkerConfiguration) -> Self {
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
