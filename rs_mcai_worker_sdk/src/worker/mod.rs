//! Module to manage the worker

use schemars::schema::RootSchema;
use schemars::schema_for;
use schemars::JsonSchema;
use semver::Version;
use serde::Deserialize;

use crate::MessageEvent;
use serde::de::DeserializeOwned;

pub mod docker;
pub mod system_information;

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Parameter {
  pub identifier: String,
  pub label: String,
  pub kind: Vec<ParameterType>,
  pub required: bool,
  // default: DefaultParameterType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerConfiguration {
  instance_id: String,
  queue_name: String,
  label: String,
  short_description: String,
  description: String,
  version: Version,
  sdk_version: Version,
  parameters: RootSchema,
}

impl WorkerConfiguration {
  pub fn new<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    queue_name: &str,
    message_event: &ME,
  ) -> Self {
    let sdk_version =
      Version::parse(built_info::PKG_VERSION).unwrap_or_else(|_| Version::new(0, 0, 0));

    let parameters = schema_for!(P);

    WorkerConfiguration {
      instance_id: docker::get_instance_id("/proc/self/cgroup"),
      queue_name: queue_name.to_string(),
      label: message_event.get_name(),
      sdk_version,
      version: message_event.get_version(),
      short_description: message_event.get_short_description(),
      description: message_event.get_description(),
      parameters,
    }
  }

  pub fn get_instance_id(&self) -> String {
    self.instance_id.clone()
  }

  pub fn get_queue_name(&self) -> String {
    self.queue_name.clone()
  }

  pub fn get_worker_name(&self) -> String {
    self.label.clone()
  }

  pub fn get_worker_version(&self) -> String {
    self.version.to_string()
  }

  pub fn get_sdk_version(&self) -> String {
    self.sdk_version.to_string()
  }

  pub fn get_consumer_mode(&self) -> String {
    "file".to_string()
  }

  pub fn get_direct_messaging_queue_name(&self) -> String {
    format!("direct_messaging_{}", self.instance_id)
  }
}
