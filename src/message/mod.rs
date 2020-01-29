mod helpers;

use crate::{
  job::{Job, JobProgression, JobResult, JobStatus},
  MessageError, MessageEvent,
};

use futures::future::Future;
use lapin_futures::{message::Delivery, options::*, BasicProperties, Channel};

static RESPONSE_EXCHANGE: &str = "job_response";
static QUEUE_JOB_COMPLETED: &str = "job_completed";
static QUEUE_JOB_ERROR: &str = "job_error";
static QUEUE_JOB_PROGRESSION: &str = "job_progression";

pub fn process_message<ME: MessageEvent>(
  message_event: &'static ME,
  message: Delivery,
  channel: &Channel,
) {
  let count = helpers::get_message_death_count(&message);
  let message_data = std::str::from_utf8(&message.data).unwrap();

  match parse_and_process_message(
    message_event,
    message_data,
    count,
    Some(channel),
    publish_job_progression,
  ) {
    Ok(job_result) => {
      info!(target: &job_result.get_str_job_id(), "Completed");
      publish_job_completed(channel, message, job_result);
    }
    Err(error) => match error {
      MessageError::RequirementsError(details) => {
        publish_missing_requirements(channel, message, &details);
      }
      MessageError::NotImplemented() => {
        publish_not_implemented(channel, message);
      }
      MessageError::ProcessingError(job_result) => {
        publish_processing_error(channel, message, job_result);
      }
      MessageError::RuntimeError(error_message) => {
        publish_runtime_error(channel, message, &error_message);
      }
    },
  }
}

pub fn parse_and_process_message<
  ME: MessageEvent,
  F: Fn(&Channel, &Job, u8) -> Result<(), MessageError> + 'static,
>(
  message_event: &'static ME,
  message_data: &str,
  count: Option<i64>,
  channel: Option<&Channel>,
  publish_job_progression: F,
) -> Result<JobResult, MessageError> {
  let job = Job::new(message_data)?;
  debug!(target: &job.job_id.to_string(),
    "received message: {:?} (iteration: {})",
    job,
    count.unwrap_or(0));

  job.check_requirements()?;

  if let Some(channel) = channel {
    publish_job_progression(channel, &job, 0)?;
  }

  let job_result = JobResult::new(job.job_id);
  MessageEvent::process(message_event, &job, job_result)
}

fn publish_job_completed(channel: &Channel, message: Delivery, job_result: JobResult) {
  let msg = json!(job_result).to_string();

  let result = channel
    .basic_publish(
      RESPONSE_EXCHANGE,
      QUEUE_JOB_COMPLETED,
      msg.as_str().as_bytes().to_vec(),
      BasicPublishOptions::default(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok();

  if result {
    if let Err(msg) = channel
      .basic_ack(message.delivery_tag, false /*not requeue*/)
      .wait()
    {
      error!(target: &job_result.get_str_job_id(), "Unable to ack message {:?}", msg);
    }
  } else if let Err(msg) = channel
    .basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
    .wait()
  {
    error!(target: &job_result.get_str_job_id(), "Unable to reject message {:?}", msg);
  }
}

fn publish_job_progression(
  channel: &Channel,
  job: &Job,
  progression: u8,
) -> Result<(), MessageError> {
  let msg = json!(JobProgression::new(job, progression)).to_string();

  channel
    .basic_publish(
      RESPONSE_EXCHANGE,
      QUEUE_JOB_PROGRESSION,
      msg.as_str().as_bytes().to_vec(),
      BasicPublishOptions::default(),
      BasicProperties::default(),
    )
    .wait()
    .map_err(|e| {
      let result = JobResult::new(job.job_id)
        .with_status(JobStatus::Error)
        .with_message(&e.to_string());
      MessageError::ProcessingError(result)
    })
}

fn publish_missing_requirements(channel: &Channel, message: Delivery, details: &str) {
  debug!("{}", details);
  if let Err(msg) = channel
    .basic_reject(message.delivery_tag, BasicRejectOptions::default())
    .wait()
  {
    error!("Unable to reject message {:?}", msg);
  }
}

fn publish_not_implemented(channel: &Channel, message: Delivery) {
  error!("Not implemented feature");
  if let Err(msg) = channel
    .basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
    .wait()
  {
    error!("Unable to reject message {:?}", msg);
  }
}

fn publish_processing_error(channel: &Channel, message: Delivery, job_result: JobResult) {
  error!(target: &job_result.get_str_job_id(), "Job returned in error: {:?}", job_result.get_parameters());

  let content = json!(JobResult::new(job_result.get_job_id())
    .with_status(JobStatus::Error)
    .with_parameters(&mut job_result.get_parameters().clone()));

  if channel
    .basic_publish(
      RESPONSE_EXCHANGE,
      QUEUE_JOB_ERROR,
      content.to_string().as_str().as_bytes().to_vec(),
      BasicPublishOptions::default(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok()
  {
    if let Err(msg) = channel
      .basic_ack(message.delivery_tag, false /*not requeue*/)
      .wait()
    {
      error!(target: &job_result.get_str_job_id(), "Unable to ack message {:?}", msg);
    }
  } else if let Err(msg) = channel
    .basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
    .wait()
  {
    error!(target: &job_result.get_str_job_id(), "Unable to reject message {:?}", msg);
  }
}

fn publish_runtime_error(channel: &Channel, message: Delivery, details: &str) {
  error!("An error occurred: {:?}", details);
  let content = json!({
    "status": "error",
    "message": details
  });
  if channel
    .basic_publish(
      RESPONSE_EXCHANGE,
      QUEUE_JOB_ERROR,
      content.to_string().as_str().as_bytes().to_vec(),
      BasicPublishOptions::default(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok()
  {
    if let Err(msg) = channel
      .basic_ack(message.delivery_tag, false /*not requeue*/)
      .wait()
    {
      error!("Unable to ack message {:?}", msg);
    }
  } else if let Err(msg) = channel
    .basic_reject(
      message.delivery_tag,
      BasicRejectOptions { requeue: true }, /*requeue*/
    )
    .wait()
  {
    error!("Unable to reject message {:?}", msg);
  }
}
