
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
  #[serde(rename = "unknown")]
  Unknown,
  #[serde(rename = "completed")]
  Completed,
  #[serde(rename = "error")]
  Error,
}

impl Default  for JobStatus {
  fn default() -> Self {
    JobStatus::Unknown
  }
}
