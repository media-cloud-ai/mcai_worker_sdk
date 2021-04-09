use crate::job::JobStatus;

/// Worker activity mode
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkerActivity {
  Idle,
  Busy,
}

impl From<JobStatus> for WorkerActivity {
  fn from(job_status: JobStatus) -> Self {
    match job_status {
      JobStatus::Initialized | JobStatus::Running => WorkerActivity::Busy,
      JobStatus::Completed | JobStatus::Stopped | JobStatus::Error | JobStatus::Unknown => {
        WorkerActivity::Idle
      }
    }
  }
}
