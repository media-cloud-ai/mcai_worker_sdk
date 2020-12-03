use crate::worker::WorkerConfiguration;
#[cfg(feature = "media")]
use crate::{Output, Source};
#[cfg(feature = "media")]
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum WorkerStatus {
  /// Waiting for a job
  READY,
  /// A job has be initialized
  INITIALIZED,
  /// Initialized and processing the job
  RUNNING,
  ///Initialized but stopped
  STOPPED,
}

#[derive(Clone)]
pub struct WorkerContext {
  pub configuration: Option<WorkerConfiguration>,
  #[cfg(feature = "media")]
  pub source: Option<Arc<Mutex<Source>>>,
  #[cfg(feature = "media")]
  pub output: Option<Arc<Mutex<Output>>>,
  pub status: WorkerStatus,
}

impl WorkerContext {
  #[cfg(feature = "media")]
  pub fn new(configuration: Option<WorkerConfiguration>) -> Self {
    WorkerContext {
      configuration,
      source: None,
      output: None,
      status: WorkerStatus::READY,
    }
  }

  #[cfg(not(feature = "media"))]
  pub fn new(configuration: Option<WorkerConfiguration>) -> Self {
    WorkerContext {
      configuration,
      status: WorkerStatus::READY,
    }
  }
}
