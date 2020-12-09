use crate::{
  job::JobStatus,
  message::{QUEUE_JOB_ERROR, RESPONSE_EXCHANGE},
  JobResult, McaiChannel,
};
use lapin::{
  message::Delivery,
  options::{BasicAckOptions, BasicPublishOptions, BasicRejectOptions},
  BasicProperties, Promise,
};

pub fn job_processing_error(
  channel: McaiChannel,
  message: &Delivery,
  job_result: &JobResult,
) -> Promise<()> {
  log::error!(target: &job_result.get_str_job_id(), "Job returned in error: {:?}", job_result.get_parameters());

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
