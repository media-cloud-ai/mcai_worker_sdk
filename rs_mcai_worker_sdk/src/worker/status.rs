use super::{SystemInformation, WorkerActivity};

/// Container to generate the status message for the worker
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerStatus {
  pub activity: WorkerActivity,
  pub system_info: SystemInformation,
}

impl WorkerStatus {
  pub fn new(activity: WorkerActivity, system_info: SystemInformation) -> Self {
    WorkerStatus {
      activity,
      system_info,
    }
  }
}
