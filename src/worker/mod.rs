use crate::MessageEvent;
use semver::Version;

#[derive(Debug, Serialize, Deserialize)]
pub enum ParameterType {
  #[serde(rename = "array_of_strings")]
  ArrayOfStrings,
  #[serde(rename = "boolean")]
  Boolean,
  #[serde(rename = "credential")]
  Credential,
  #[serde(rename = "integer")]
  Integer,
  #[serde(rename = "requirement")]
  Requirement,
  #[serde(rename = "string")]
  String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameter {
  pub identifier: String,
  pub label: String,
  pub kind: Vec<ParameterType>,
  pub required: bool,
  // default: DefaultParameterType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerConfiguration {
  queue_name: String,
  label: String,
  short_description: String,
  description: String,
  version: Version,
  git_version: Version,
  parameters: Vec<Parameter>
}

impl WorkerConfiguration {
  pub fn new<ME: MessageEvent>(queue_name: &str, message_event: &'static ME) -> Self {
    WorkerConfiguration {
      queue_name: queue_name.to_string(),
      label: message_event.get_name(),
      version: message_event.get_version(),
      short_description: message_event.get_short_description(),
      description: message_event.get_description(),
      git_version: message_event.get_git_version(),
      parameters: message_event.get_parameters()
    }
  }

  pub fn add_parameter(&mut self, parameter: Parameter) {
    self.parameters.push(parameter);
  }
}
