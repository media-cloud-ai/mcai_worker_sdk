mod job_missing_requirements;
mod job_not_implemented;
mod job_parameter_error;
mod job_progression;
mod publish_job_response;
mod publish_worker_response;

pub use job_missing_requirements::job_missing_requirements;
pub use job_not_implemented::job_not_implemented;
pub use job_parameter_error::job_parameter_error;
pub use job_progression::job_progression;
pub use publish_job_response::publish_job_response;
pub use publish_worker_response::publish_worker_response;

use crate::{
  job::{JobResult, JobStatus},
  message_exchange::{
    message::{Feedback, ResponseMessage},
    rabbitmq::{
      QUEUE_JOB_COMPLETED, QUEUE_JOB_ERROR, QUEUE_JOB_STOPPED, QUEUE_WORKER_CREATED, QUEUE_WORKER_INITIALIZED,
      QUEUE_WORKER_STARTED, QUEUE_WORKER_STATUS,
    },
  },
  MessageError, Result,
};
use lapin::{message::Delivery, Channel};
use std::sync::Arc;

pub async fn response_with_delivery(
  channel: Arc<Channel>,
  delivery: Option<Delivery>,
  response: &ResponseMessage,
) -> Result<()> {
  match response {
    ResponseMessage::WorkerCreated(worker_configuration) => {
      let payload = json!(worker_configuration).to_string();

      publish_worker_response(channel, delivery, QUEUE_WORKER_CREATED, &payload).await
    }
    ResponseMessage::WorkerInitialized(job_result) => {
      let payload = json!(job_result).to_string();

      publish_worker_response(channel, delivery, QUEUE_WORKER_INITIALIZED, &payload).await
    }
    ResponseMessage::WorkerStarted(job_result) => {
      let payload = json!(job_result).to_string();

      publish_worker_response(channel, delivery, QUEUE_WORKER_STARTED, &payload).await
    }
    ResponseMessage::Completed(job_result) => {
      let payload = json!(job_result).to_string();

      if delivery.is_none() {
        return Err(MessageError::RuntimeError(
          "Cannot send response without corresponding delivery.".to_string(),
        ));
      }

      publish_job_response(channel, &delivery.unwrap(), QUEUE_JOB_COMPLETED, &payload).await
    }
    ResponseMessage::Error(message_error) => {
      if delivery.is_none() {
        return Err(MessageError::RuntimeError(
          "Cannot send response without corresponding delivery.".to_string(),
        ));
      }

      error(channel, &delivery.unwrap(), message_error).await
    }
    ResponseMessage::JobStopped(job_result) => {
      let payload = json!(job_result).to_string();

      if delivery.is_none() {
        return Err(MessageError::RuntimeError(
          "Cannot send response without corresponding delivery.".to_string(),
        ));
      }

      publish_job_response(channel, &delivery.unwrap(), QUEUE_JOB_STOPPED, &payload).await
    },
    ResponseMessage::Feedback(feedback) => match feedback {
      Feedback::Progression(progression) => job_progression(channel, progression.clone()),
      Feedback::Status(_process_status) => {
        let payload = json!(feedback).to_string();

        publish_worker_response(channel, delivery, QUEUE_WORKER_STATUS, &payload).await
      }
    },
    ResponseMessage::StatusError(message_error) => {
      if delivery.is_none() {
        return Err(MessageError::RuntimeError(
          "Cannot send response without corresponding delivery.".to_string(),
        ));
      }

      error(channel, &delivery.unwrap(), message_error).await
    }
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
      log::error!(target: &job_result.get_str_job_id(), "Job returned in error: {:?}", job_result.get_parameters());

      let job_result = JobResult::new(job_result.get_job_id())
        .with_status(JobStatus::Error)
        .with_parameters(&mut job_result.get_parameters().clone());

      let payload = json!(job_result).to_string();

      publish_job_response(channel, delivery, QUEUE_JOB_ERROR, &payload).await
    }
    MessageError::RuntimeError(error_message) => {
      log::error!("An error occurred: {:?}", error_message);
      let payload = json!({
        "status": "error",
        "message": error_message
      })
      .to_string();

      publish_job_response(channel, delivery, QUEUE_JOB_ERROR, &payload).await
    }
  }
}
