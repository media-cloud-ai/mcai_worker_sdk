use crate::job::Job;
use chrono::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct JobProgression {
  datetime: DateTime<Utc>,
  job_id: u64,
  progression: u8,
}

impl JobProgression {
  pub fn new(job: &Job, progression: u8) -> Self {
    JobProgression {
      datetime: Utc::now(),
      job_id: job.job_id,
      progression,
    }
  }
}
