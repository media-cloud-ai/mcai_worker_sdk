use crate::worker::docker::get_instance_id;
use chrono::prelude::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobProgression {
  datetime: DateTime<Utc>,
  docker_container_id: String,
  pub job_id: u64,
  pub progression: u8,
}

impl JobProgression {
  pub fn new(job_id: u64, progression: u8) -> Self {
    JobProgression {
      datetime: Utc::now(),
      docker_container_id: get_instance_id("/proc/self/cgroup"),
      job_id,
      progression,
    }
  }
}

#[test]
pub fn test_job_progression() {
  let job_id: u64 = 123;
  let progression: u8 = 25;
  let date_format = "%Y %b %d %H:%M:%S";
  let now = Utc::now();

  let job_progression = JobProgression::new(job_id, progression);

  assert_eq!(job_id, job_progression.job_id);
  assert_eq!(progression, job_progression.progression);
  assert_eq!(
    now.format(date_format).to_string(),
    job_progression.datetime.format(date_format).to_string()
  );
  assert!(!job_progression.docker_container_id.is_empty());
}
