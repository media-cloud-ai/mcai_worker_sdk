use crate::{EbuTtmlLive, Frame};

pub enum ProcessFrame {
  AudioVideo(Frame),
  EbuTtmlLive(Box<EbuTtmlLive>),
  Data(Vec<u8>),
}

impl ProcessFrame {
  pub fn get_pts(&self) -> i64 {
    match self {
      ProcessFrame::AudioVideo(frame) => frame.get_pts(),
      ProcessFrame::EbuTtmlLive(_) | ProcessFrame::Data(_) => {
        // improvement: support pts to terminate
        0
      }
    }
  }
}
