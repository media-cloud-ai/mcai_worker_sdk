use crate::{job::JobResult, worker::WorkerStatus};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessStatus {
  // A `JobResult` that contains a job_status
  pub job: Option<JobResult>,
  pub worker: WorkerStatus,
}

impl ProcessStatus {
  pub fn new(worker_status: WorkerStatus, job_result: Option<JobResult>) -> Self {
    ProcessStatus {
      job: job_result,
      worker: worker_status,
    }
  }
}
