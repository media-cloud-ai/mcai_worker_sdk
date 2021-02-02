use crate::job::JobProgression;
use crate::processor::ProcessStatus;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Feedback {
  Progression(JobProgression),
  Status(ProcessStatus),
}
