use crate::{
  info,
  job::{JobProgression, JobResult, JobStatus},
  message_exchange::rabbitmq::{QUEUE_JOB_PROGRESSION, RESPONSE_EXCHANGE},
  MessageError, Result,
};
use lapin::{options::BasicPublishOptions, BasicProperties, Channel};
use std::sync::Arc;

/// Function to publish a progression event
///
/// It will be an integer between 0 and 100.
pub fn job_progression(
  channel: Option<Arc<Channel>>,
  job_progression: JobProgression,
) -> Result<()> {
  if let Some(channel) = channel {
    let msg = json!(job_progression).to_string();

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
        let result = JobResult::new(job_progression.job_id)
          .with_status(JobStatus::Error)
          .with_message(&e.to_string());
        MessageError::ProcessingError(result)
      })
      .map(|_| ())
  } else {
    info!(target: &job_progression.job_id.to_string(), "progression: {}%", job_progression.progression);
    Ok(())
  }
}
