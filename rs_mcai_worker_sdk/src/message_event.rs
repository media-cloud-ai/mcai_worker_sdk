use crate::{JobResult, McaiChannel, MessageError, Result};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

#[cfg(feature = "media")]
use crate::{FormatContext, ProcessFrame, ProcessResult, StreamDescriptor};
#[cfg(feature = "media")]
use std::sync::{Arc, Mutex, mpsc::Sender};

/// # Trait to describe a worker
/// Implement this trait to implement a worker
pub trait MessageEvent<P: DeserializeOwned + JsonSchema> {
  fn get_name(&self) -> String;
  fn get_short_description(&self) -> String;
  fn get_description(&self) -> String;
  fn get_version(&self) -> semver::Version;

  fn init(&mut self) -> Result<()> {
    Ok(())
  }

  #[cfg(feature = "media")]
  fn init_process(
    &mut self,
    _parameters: P,
    _format_context: Arc<Mutex<FormatContext>>,
    _response_sender: Arc<Mutex<Sender<ProcessResult>>>,
  ) -> Result<Vec<StreamDescriptor>> {
    Ok(vec![])
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &mut self,
    _job_result: JobResult,
    _stream_index: usize,
    _frame: ProcessFrame,
  ) -> Result<ProcessResult> {
    Err(MessageError::NotImplemented())
  }

  #[cfg(feature = "media")]
  fn ending_process(&mut self) -> Result<()> {
    Ok(())
  }

  /// Not called when the "media" feature is enabled
  fn process(
    &self,
    _channel: Option<McaiChannel>,
    _parameters: P,
    _job_result: JobResult,
  ) -> Result<JobResult>
  where
    Self: std::marker::Sized,
  {
    Err(MessageError::NotImplemented())
  }
}
