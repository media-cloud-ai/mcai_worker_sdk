#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
  #[serde(rename = "unknown")]
  Unknown,
  #[serde(rename = "completed")]
  Completed,
  #[serde(rename = "error")]
  Error,
}

impl Default for JobStatus {
  fn default() -> Self {
    JobStatus::Unknown
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
