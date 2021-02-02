use crate::worker::WorkerActivity;
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
  Unknown,
  Initialized,
  Running,
  Stopped,
  Completed,
  Error,
}

impl Default for JobStatus {
  fn default() -> Self {
    JobStatus::Unknown
  }
}

impl fmt::Display for JobStatus {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl Into<WorkerActivity> for JobStatus {
  fn into(self) -> WorkerActivity {
    match self {
      JobStatus::Initialized | JobStatus::Running => WorkerActivity::Busy,
      JobStatus::Completed | JobStatus::Stopped | JobStatus::Error | JobStatus::Unknown => {
        WorkerActivity::Idle
      }
    }
  }
}

#[test]
pub fn test_job_status_json() {
  let json = serde_json::to_string(&JobStatus::Unknown).unwrap();
  assert_eq!("\"unknown\"", &json);
  let json = serde_json::to_string(&JobStatus::Completed).unwrap();
  assert_eq!("\"completed\"", &json);
  let json = serde_json::to_string(&JobStatus::Error).unwrap();
  assert_eq!("\"error\"", &json);
}
