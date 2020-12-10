#[cfg(feature = "media")]
pub mod media;

#[cfg(feature = "media")]
pub use media::{DESTINATION_PATH_PARAMETER, SOURCE_PATH_PARAMETER};

use crate::{job::JobProgression, message_exchange::Feedback, McaiChannel, MessageError, Result};
use async_std::task;

/// Function to publish a progression event
///
/// It will be an integer between 0 and 100.
pub fn publish_job_progression(
  channel: Option<McaiChannel>,
  job_id: u64,
  progression: u8,
) -> Result<()> {
  if let Some(channel) = channel {
    task::block_on(async move {
      channel
        .send(Feedback::Progression(JobProgression::new(
          job_id,
          progression,
        )))
        .await
        .map_err(|e| MessageError::RuntimeError(e.to_string()))
    })
  } else {
    info!(target: &job_id.to_string(), "progression: {}%", progression);
    Ok(())
  }
}
