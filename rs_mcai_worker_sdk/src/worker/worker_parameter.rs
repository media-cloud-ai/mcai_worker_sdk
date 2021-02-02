use super::WorkerParameterType;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerParameter {
  pub identifier: String,
  pub label: String,
  pub kind: Vec<WorkerParameterType>,
  pub required: bool,
  // default: DefaultParameterType,
}
