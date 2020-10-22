//! Module to manage the worker

use schemars::schema::RootSchema;
use schemars::schema_for;
use schemars::JsonSchema;
use semver::Version;
use serde::Deserialize;

#[cfg(feature = "media")]
use crate::{
  message::{DESTINATION_PATH_PARAMETER, SOURCE_PATH_PARAMETER},
  MessageError,
};
use crate::{MessageEvent, Result};
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
  #[serde(rename = "requirements")]
  Requirements,
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
    instance_id: &str,
  ) -> Result<Self> {
    let sdk_version =
      Version::parse(built_info::PKG_VERSION).unwrap_or_else(|_| Version::new(0, 0, 0));

    let parameters = WorkerConfiguration::get_parameter_schema::<P>()?;

    Ok(WorkerConfiguration {
      instance_id: instance_id.to_string(),
      queue_name: queue_name.to_string(),
      label: message_event.get_name(),
      sdk_version,
      version: message_event.get_version(),
      short_description: message_event.get_short_description(),
      description: message_event.get_description(),
      parameters,
    })
  }

  #[cfg(feature = "media")]
  fn get_parameter_schema<P: JsonSchema>() -> Result<RootSchema> {
    let mut parameters: RootSchema = schema_for!(P);
    if !parameters
      .schema
      .object()
      .properties
      .contains_key(SOURCE_PATH_PARAMETER)
    {
      return Err(MessageError::ParameterValueError(format!(
        "Expected media parameter missing: '{}'",
        SOURCE_PATH_PARAMETER
      )));
    }
    if !parameters
      .schema
      .object()
      .properties
      .contains_key(DESTINATION_PATH_PARAMETER)
    {
      return Err(MessageError::ParameterValueError(format!(
        "Expected media parameter missing: '{}'",
        DESTINATION_PATH_PARAMETER
      )));
    }
    Ok(parameters)
  }

  #[cfg(not(feature = "media"))]
  fn get_parameter_schema<P: JsonSchema>() -> Result<RootSchema> {
    Ok(schema_for!(P))
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
