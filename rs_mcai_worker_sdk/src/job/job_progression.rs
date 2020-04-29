use crate::job::Job;
use crate::worker::docker::get_instance_id;
use chrono::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct JobProgression {
  datetime: DateTime<Utc>,
  docker_container_id: String,
  job_id: u64,
  progression: u8,
}

impl JobProgression {
  pub fn new(job: &Job, progression: u8) -> Self {
    JobProgression {
      datetime: Utc::now(),
      docker_container_id: get_instance_id("/proc/self/cgroup"),
      job_id: job.job_id,
      progression,
    }
  }
}
