extern crate assert_matches;
extern crate mcai_worker_sdk;
#[macro_use]
extern crate serde_derive;

use mcai_worker_sdk::prelude::*;

use schemars::JsonSchema;

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[test]
#[cfg(not(feature = "media"))]
pub fn test_worker_configuration_new() {
  let queue_name = "queue_name".to_string();
  let instance_id = "instance_id".to_string();
  let default_consumer_mode = "file".to_string();
  let sdk_version =
    Version::parse(built_info::PKG_VERSION).unwrap_or_else(|_| Version::new(0, 0, 0));

  #[derive(Debug)]
  struct CustomEvent {}

  #[derive(JsonSchema, Deserialize)]
  struct CustomParameters {}

  impl MessageEvent<CustomParameters> for CustomEvent {
    fn get_name(&self) -> String {
      "worker name".to_string()
    }
    fn get_short_description(&self) -> String {
      "short description".to_string()
    }
    fn get_description(&self) -> String {
      "long description".to_string()
    }
    fn get_version(&self) -> semver::Version {
      semver::Version::new(1, 2, 3)
    }
  }

  let message_event = CustomEvent {};

  let result = WorkerConfiguration::new(&queue_name, &message_event, &instance_id);
  assert!(result.is_ok());

  let worker_configuration = result.unwrap();

  assert_eq!(queue_name, worker_configuration.get_queue_name());
  assert_eq!(instance_id, worker_configuration.get_instance_id());
  assert_eq!(
    default_consumer_mode,
    worker_configuration.get_consumer_mode()
  );
  assert_eq!(
    format!("direct_messaging_{}", instance_id),
    worker_configuration.get_direct_messaging_queue_name()
  );
  assert_eq!(
    sdk_version.to_string(),
    worker_configuration.get_sdk_version()
  );
  assert_eq!(
    "worker name".to_string(),
    worker_configuration.get_worker_name()
  );
  assert_eq!(
    "1.2.3".to_string().to_string(),
    worker_configuration.get_worker_version()
  );
}

#[test]
#[cfg(feature = "media")]
pub fn test_media_worker_configuration_new() {
  let queue_name = "queue_name".to_string();
  let instance_id = "instance_id".to_string();
  let default_consumer_mode = "file".to_string();
  let sdk_version =
    Version::parse(built_info::PKG_VERSION).unwrap_or_else(|_| Version::new(0, 0, 0));

  #[derive(Debug)]
  struct CustomEvent {}

  #[derive(JsonSchema, Deserialize)]
  struct CustomParameters {
    #[allow(dead_code)]
    source_path: String,
    #[allow(dead_code)]
    destination_path: String,
  }

  impl MessageEvent<CustomParameters> for CustomEvent {
    fn get_name(&self) -> String {
      "worker name".to_string()
    }
    fn get_short_description(&self) -> String {
      "short description".to_string()
    }
    fn get_description(&self) -> String {
      "long description".to_string()
    }
    fn get_version(&self) -> semver::Version {
      semver::Version::new(1, 2, 3)
    }
  }

  let message_event = CustomEvent {};

  assert_eq!("worker name".to_string(), message_event.get_name());
  assert_eq!(
    "short description".to_string(),
    message_event.get_short_description()
  );
  assert_eq!(
    "long description".to_string(),
    message_event.get_description()
  );
  assert_eq!(semver::Version::new(1, 2, 3), message_event.get_version());

  let result = WorkerConfiguration::new(&queue_name, &message_event, &instance_id);
  assert!(result.is_ok());

  let worker_configuration = result.unwrap();

  assert_eq!(queue_name, worker_configuration.get_queue_name());
  assert_eq!(instance_id, worker_configuration.get_instance_id());
  assert_eq!(
    default_consumer_mode,
    worker_configuration.get_consumer_mode()
  );
  assert_eq!(
    format!("direct_messaging_{}", instance_id),
    worker_configuration.get_direct_messaging_queue_name()
  );
  assert_eq!(
    sdk_version.to_string(),
    worker_configuration.get_sdk_version()
  );
  assert_eq!(
    "worker name".to_string(),
    worker_configuration.get_worker_name()
  );
  assert_eq!(
    "1.2.3".to_string().to_string(),
    worker_configuration.get_worker_version()
  );
}

#[test]
#[cfg(feature = "media")]
pub fn test_media_worker_configuration_new_missing_source_parameter() {
  let queue_name = "queue_name".to_string();
  let instance_id = "instance_id".to_string();

  #[derive(Debug)]
  struct CustomEvent {}

  #[derive(JsonSchema, Deserialize)]
  struct CustomParameters {}

  impl MessageEvent<CustomParameters> for CustomEvent {
    fn get_name(&self) -> String {
      "worker name".to_string()
    }
    fn get_short_description(&self) -> String {
      "short description".to_string()
    }
    fn get_description(&self) -> String {
      "long description".to_string()
    }
    fn get_version(&self) -> semver::Version {
      semver::Version::new(1, 2, 3)
    }
  }

  let message_event = CustomEvent {};

  assert_eq!("worker name".to_string(), message_event.get_name());
  assert_eq!(
    "short description".to_string(),
    message_event.get_short_description()
  );
  assert_eq!(
    "long description".to_string(),
    message_event.get_description()
  );
  assert_eq!(semver::Version::new(1, 2, 3), message_event.get_version());

  let result = WorkerConfiguration::new(&queue_name, &message_event, &instance_id);
  let expected = MessageError::ParameterValueError(
    "Expected media parameter missing: 'source_path'".to_string(),
  );

  assert!(result.is_err());
  assert_eq!(expected, result.unwrap_err());
}

#[test]
#[cfg(feature = "media")]
pub fn test_media_worker_configuration_new_missing_destination_parameter() {
  let queue_name = "queue_name".to_string();
  let instance_id = "instance_id".to_string();

  #[derive(Debug)]
  struct CustomEvent {}

  #[derive(JsonSchema, Deserialize)]
  struct CustomParameters {
    #[allow(dead_code)]
    source_path: String,
  }

  impl MessageEvent<CustomParameters> for CustomEvent {
    fn get_name(&self) -> String {
      "worker name".to_string()
    }
    fn get_short_description(&self) -> String {
      "short description".to_string()
    }
    fn get_description(&self) -> String {
      "long description".to_string()
    }
    fn get_version(&self) -> semver::Version {
      semver::Version::new(1, 2, 3)
    }
  }

  let message_event = CustomEvent {};

  assert_eq!("worker name".to_string(), message_event.get_name());
  assert_eq!(
    "short description".to_string(),
    message_event.get_short_description()
  );
  assert_eq!(
    "long description".to_string(),
    message_event.get_description()
  );
  assert_eq!(semver::Version::new(1, 2, 3), message_event.get_version());

  let result = WorkerConfiguration::new(&queue_name, &message_event, &instance_id);
  let expected = MessageError::ParameterValueError(
    "Expected media parameter missing: 'destination_path'".to_string(),
  );

  assert!(result.is_err());
  assert_eq!(expected, result.unwrap_err());
}

#[test]
#[cfg(feature = "media")]
pub fn test_media_worker_configuration_new_missing_parameters() {
  let queue_name = "queue_name".to_string();
  let instance_id = "instance_id".to_string();

  #[derive(Debug)]
  struct CustomEvent {}

  #[derive(JsonSchema, Deserialize)]
  struct CustomParameters {
    #[allow(dead_code)]
    other: String,
  }

  impl MessageEvent<CustomParameters> for CustomEvent {
    fn get_name(&self) -> String {
      "worker name".to_string()
    }
    fn get_short_description(&self) -> String {
      "short description".to_string()
    }
    fn get_description(&self) -> String {
      "long description".to_string()
    }
    fn get_version(&self) -> semver::Version {
      semver::Version::new(1, 2, 3)
    }
  }

  let message_event = CustomEvent {};

  assert_eq!("worker name".to_string(), message_event.get_name());
  assert_eq!(
    "short description".to_string(),
    message_event.get_short_description()
  );
  assert_eq!(
    "long description".to_string(),
    message_event.get_description()
  );
  assert_eq!(semver::Version::new(1, 2, 3), message_event.get_version());

  let result = WorkerConfiguration::new(&queue_name, &message_event, &instance_id);
  let expected = MessageError::ParameterValueError(
    "Expected media parameter missing: 'source_path'".to_string(),
  );

  assert!(result.is_err());
  assert_eq!(expected, result.unwrap_err());
}
