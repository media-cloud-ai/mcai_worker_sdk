#[cfg(feature = "media")]
pub use media::{DESTINATION_PATH_PARAMETER, SOURCE_PATH_PARAMETER};

use crate::message_exchange::Feedback;
use crate::message_exchange::ResponseMessage;
use crate::{job::JobProgression, McaiChannel, Result};

#[cfg(feature = "media")]
pub mod media;

/// Function to publish a progression event
///
/// It will be an integer between 0 and 100.
pub fn publish_job_progression(
  channel: Option<McaiChannel>,
  job_id: u64,
  progression: u8,
) -> Result<()> {
  if let Some(response_channel) = channel {
    return response_channel
      .lock()
      .unwrap()
      .send_response(ResponseMessage::Feedback(Feedback::Progression(
        JobProgression::new(job_id, progression),
      )));
  }
  Ok(())
}
