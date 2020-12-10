pub mod feedback_publisher;
mod job_completed;
mod job_missing_requirements;
mod job_not_implemented;
mod job_parameter_error;
mod job_processing_error;
mod job_progression;
mod job_runtime_error;

pub use job_completed::job_completed;
pub use job_missing_requirements::job_missing_requirements;
pub use job_not_implemented::job_not_implemented;
pub use job_parameter_error::job_parameter_error;
pub use job_processing_error::job_processing_error;
pub use job_runtime_error::job_runtime_error;

use crate::{message_exchange::ResponseMessage, MessageError, Result};
use lapin::{message::Delivery, Channel};
use std::sync::Arc;

pub async fn response(
  channel: Arc<Channel>,
  delivery: &Delivery,
  response: &ResponseMessage,
) -> Result<()> {
  match response {
    ResponseMessage::Completed(job_result) => {
      log::info!(target: &job_result.get_str_job_id(), "Response: {:?}", job_result);
      job_completed(channel, delivery, job_result)
        .await
        .map_err(|e| e.into())
    }
    ResponseMessage::Error(message_error) => error(channel, delivery, message_error).await,
    ResponseMessage::Initialized => Ok(()),
    ResponseMessage::Progression(_job_id, _progression) => Ok(()),
  }
}

pub async fn error(channel: Arc<Channel>, delivery: &Delivery, error: &MessageError) -> Result<()> {
  match error {
    MessageError::Amqp(_lapin_error) => unimplemented!(),
    MessageError::RequirementsError(details) => {
      job_missing_requirements(channel, delivery, &details)
        .await
        .map_err(|e| e.into())
    }
    MessageError::NotImplemented() => job_not_implemented(channel, delivery)
      .await
      .map_err(|e| e.into()),
    MessageError::ParameterValueError(error_message) => {
      job_parameter_error(channel, delivery, &error_message)
        .await
        .map_err(|e| e.into())
    }
    MessageError::ProcessingError(job_result) => {
      job_processing_error(channel, delivery, job_result)
        .await
        .map_err(|e| e.into())
    }
    MessageError::RuntimeError(error_message) => {
      job_runtime_error(channel, delivery, &error_message)
        .await
        .map_err(|e| e.into())
    }
  }
}
