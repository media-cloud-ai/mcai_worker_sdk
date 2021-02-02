use super::ParameterType;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerParameter {
  pub identifier: String,
  pub label: String,
  pub kind: Vec<ParameterType>,
  pub required: bool,
  // default: DefaultParameterType,
}
