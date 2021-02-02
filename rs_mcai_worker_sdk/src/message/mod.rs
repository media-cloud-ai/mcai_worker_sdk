#[cfg(feature = "media")]
pub use media::{DESTINATION_PATH_PARAMETER, SOURCE_PATH_PARAMETER};

use crate::{McaiChannel, Result};

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
      .progression(job_id, progression);
  }
  Ok(())
}
