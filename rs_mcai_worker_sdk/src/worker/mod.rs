//! Module to manage the worker

use serde::Deserialize;

pub mod configuration;
pub mod docker;
pub mod status;
pub mod system_information;

pub use configuration::WorkerConfiguration;

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
