#[cfg(feature = "media")]
pub mod media;

#[cfg(feature = "media")]
pub use media::{DESTINATION_PATH_PARAMETER, SOURCE_PATH_PARAMETER};

use crate::{
  job::{JobProgression, JobResult, JobStatus},
  McaiChannel, MessageError, Result,
};
use lapin::{options::*, BasicProperties};

pub static RESPONSE_EXCHANGE: &str = "job_response";
pub static QUEUE_JOB_COMPLETED: &str = "job_completed";
pub static QUEUE_JOB_ERROR: &str = "job_error";
pub static QUEUE_JOB_PROGRESSION: &str = "job_progression";

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
