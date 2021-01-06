#[cfg(feature = "media")]
use crate::message::{DESTINATION_PATH_PARAMETER, SOURCE_PATH_PARAMETER};
#[cfg(feature = "media")]
use crate::MessageError;
use crate::{MessageEvent, Version};
use schemars::schema::RootSchema;
use schemars::schema_for;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
    // TODO should be optional
    queue_name: &str,
    message_event: &ME,
    instance_id: &str,
  ) -> crate::Result<Self> {
    let sdk_version =
      Version::parse(super::built_info::PKG_VERSION).unwrap_or_else(|_| Version::new(0, 0, 0));

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
    "file".to_string()
  }

  pub fn get_direct_messaging_queue_name(&self) -> String {
    format!("direct_messaging_{}", self.instance_id)
  }
}
