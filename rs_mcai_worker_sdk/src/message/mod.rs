pub(crate) mod control;

mod helpers;
#[cfg(feature = "media")]
pub mod media;

#[cfg(feature = "media")]
pub use media::{DESTINATION_PATH_PARAMETER, SOURCE_PATH_PARAMETER};

use crate::{
  job::{Job, JobProgression, JobResult, JobStatus},
  McaiChannel, MessageError, MessageEvent, Result,
};
use lapin::{message::Delivery, options::*, BasicProperties, Promise};

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::rc::Rc;

static RESPONSE_EXCHANGE: &str = "job_response";
static QUEUE_JOB_COMPLETED: &str = "job_completed";
static QUEUE_JOB_ERROR: &str = "job_error";
static QUEUE_JOB_PROGRESSION: &str = "job_progression";

pub fn process_message<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  message_event: Rc<RefCell<ME>>,
  message: Delivery,
  channel: McaiChannel,
) -> Promise<()> {
  let count = helpers::get_message_death_count(&message);
  let message_data = std::str::from_utf8(&message.data).unwrap();

  match parse_and_process_message(
    message_event,
    message_data,
    count,
    Some(channel.clone()),
    publish_job_progression,
  ) {
    Ok(job_result) => {
      info!(target: &job_result.get_str_job_id(), "Completed");
      publish_job_completed(channel, message, job_result)
    }
    Err(error) => match error {
      MessageError::RequirementsError(details) => {
        publish_missing_requirements(channel, message, &details)
      }
      MessageError::NotImplemented() => publish_not_implemented(channel, message),
      MessageError::ParameterValueError(error_message) => {
        publish_parameter_error(channel, message, &error_message)
      }
      MessageError::ProcessingError(job_result) => {
        publish_processing_error(channel, message, job_result)
      }
      MessageError::RuntimeError(error_message) => {
        publish_runtime_error(channel, message, &error_message)
      }
    },
  }
}

pub fn parse_and_process_message<
  P: DeserializeOwned + JsonSchema,
  ME: MessageEvent<P>,
  F: Fn(Option<McaiChannel>, u64, u8) -> Result<()> + 'static,
>(
  message_event: Rc<RefCell<ME>>,
  message_data: &str,
  count: Option<i64>,
  channel: Option<McaiChannel>,
  publish_job_progression: F,
) -> Result<JobResult> {
  let job = Job::new(message_data)?;
  debug!(target: &job.job_id.to_string(),
         "received message: {:?} (iteration: {})",
         job,
         count.unwrap_or(0));

  job.check_requirements()?;
  let parameters: P = job.get_parameters()?;

  publish_job_progression(channel.clone(), job.job_id, 0)?;

  let job_result = JobResult::new(job.job_id);

  #[cfg(feature = "media")]
  return media::process(message_event, channel, &job, parameters, job_result);

  #[cfg(not(feature = "media"))]
  message_event
    .borrow_mut()
    .process(channel, parameters, job_result)
}

fn publish_job_completed(
  channel: McaiChannel,
  message: Delivery,
  job_result: JobResult,
) -> Promise<()> {
  let msg = json!(job_result).to_string();

  let result = channel
    .basic_publish(
      RESPONSE_EXCHANGE,
      QUEUE_JOB_COMPLETED,
      BasicPublishOptions::default(),
      msg.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok();

  if result {
    channel.basic_ack(
      message.delivery_tag,
      BasicAckOptions::default(), /*not requeue*/
    )
  } else {
    channel.basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
  }
}

/// Function to publish a progression event
///
/// It will be an integer between 0 and 100.
pub fn publish_job_progression(
  channel: Option<McaiChannel>,
  job_id: u64,
  progression: u8,
) -> Result<()> {
  if let Some(channel) = channel {
    let msg = json!(JobProgression::new(job_id, progression)).to_string();

    channel
      .basic_publish(
        RESPONSE_EXCHANGE,
        QUEUE_JOB_PROGRESSION,
        BasicPublishOptions::default(),
        msg.as_bytes().to_vec(),
        BasicProperties::default(),
      )
      .wait()
      .map_err(|e| {
        let result = JobResult::new(job_id)
          .with_status(JobStatus::Error)
          .with_message(&e.to_string());
        MessageError::ProcessingError(result)
      })
      .map(|_| ())
  } else {
    info!(target: &job_id.to_string(), "progression: {}%", progression);
    Ok(())
  }
}

fn publish_missing_requirements(
  channel: McaiChannel,
  message: Delivery,
  details: &str,
) -> Promise<()> {
  debug!("{}", details);
  channel.basic_reject(message.delivery_tag, BasicRejectOptions::default())
}

fn publish_not_implemented(channel: McaiChannel, message: Delivery) -> Promise<()> {
  error!("Not implemented feature");
  channel.basic_reject(
    message.delivery_tag,
    BasicRejectOptions { requeue: true }, /*requeue*/
  )
}

fn publish_parameter_error(channel: McaiChannel, message: Delivery, details: &str) -> Promise<()> {
  debug!("Parameter value error: {}", details);
  channel.basic_reject(message.delivery_tag, BasicRejectOptions::default())
}

fn publish_processing_error(
  channel: McaiChannel,
  message: Delivery,
  job_result: JobResult,
) -> Promise<()> {
  error!(target: &job_result.get_str_job_id(), "Job returned in error: {:?}", job_result.get_parameters());

  let content = json!(JobResult::new(job_result.get_job_id())
    .with_status(JobStatus::Error)
    .with_parameters(&mut job_result.get_parameters().clone()))
  .to_string();

  if channel
    .basic_publish(
      RESPONSE_EXCHANGE,
      QUEUE_JOB_ERROR,
      BasicPublishOptions::default(),
      content.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok()
  {
    channel.basic_ack(
      message.delivery_tag,
      BasicAckOptions::default(), /*not requeue*/
    )
  } else {
    channel.basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
  }
}

fn publish_runtime_error(channel: McaiChannel, message: Delivery, details: &str) -> Promise<()> {
  error!("An error occurred: {:?}", details);
  let content = json!({
    "status": "error",
    "message": details
  })
  .to_string();

  if channel
    .basic_publish(
      RESPONSE_EXCHANGE,
      QUEUE_JOB_ERROR,
      BasicPublishOptions::default(),
      content.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok()
  {
    channel.basic_ack(
      message.delivery_tag,
      BasicAckOptions::default(), /*not requeue*/
    )
  } else {
    channel.basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
  }
}
