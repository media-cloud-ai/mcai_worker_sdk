use crate::MessageEvent;
#[cfg(feature = "media")]
use crate::{
  message::{DESTINATION_PATH_PARAMETER, SOURCE_PATH_PARAMETER},
  MessageError,
};
use schemars::{schema::RootSchema, schema_for, JsonSchema};
use semver::Version;
use serde::de::DeserializeOwned;

/// Structure that contains configuration for that worker
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkerConfiguration {
  instance_id: String,
  queue_name: String,
  direct_messaging_queue_name: String,
  label: String,
  short_description: String,
  description: String,
  version: Version,
  sdk_version: Version,
  parameters: RootSchema,
}

impl WorkerConfiguration {
  pub fn new<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    // TODO should be optional
    queue_name: &str,
    message_event: &ME,
    instance_id: &str,
  ) -> crate::Result<Self> {
    let sdk_version =
      Version::parse(super::built_info::PKG_VERSION).unwrap_or_else(|_| Version::new(0, 0, 0));

    let parameters = WorkerConfiguration::get_parameter_schema::<P>()?;

    let identifier =
      std::env::var("DIRECT_MESSAGING_IDENTIFIER").unwrap_or_else(|_| instance_id.to_string());

    let direct_messaging_queue_name = format!("direct_messaging_{}", identifier);

    Ok(WorkerConfiguration {
      instance_id: instance_id.to_string(),
      queue_name: queue_name.to_string(),
      direct_messaging_queue_name,
      label: message_event.get_name(),
      sdk_version,
      version: message_event.get_version(),
      short_description: message_event.get_short_description(),
      description: message_event.get_description(),
      parameters,
    })
  }

  #[cfg(feature = "media")]
  fn get_parameter_schema<P: JsonSchema>() -> crate::Result<RootSchema> {
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
  fn get_parameter_schema<P: JsonSchema>() -> crate::Result<RootSchema> {
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
    // TODO if it still necessary ?
    "file".to_string()
  }

  pub fn get_direct_messaging_queue_name(&self) -> String {
    self.direct_messaging_queue_name.clone()
  }
}
