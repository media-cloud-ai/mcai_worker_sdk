use crate::message::media::ebu_ttml_live::EbuTtmlLive;
use crate::message::media::json::Json;
use stainless_ffmpeg::prelude::*;

pub enum ProcessFrame {
  AudioVideo(Frame),
  EbuTtmlLive(Box<EbuTtmlLive>),
  Json(Box<Json>),
  Data(Vec<u8>),
}

impl Drop for ProcessFrame {
  fn drop(&mut self) {
    if let ProcessFrame::AudioVideo(frame) = self {
      unsafe {
        av_frame_free(&mut frame.frame);
      }
    }
  }
}

impl ProcessFrame {
  pub fn get_pts(&self) -> i64 {
    match self {
      ProcessFrame::AudioVideo(frame) => frame.get_pts(),
      ProcessFrame::EbuTtmlLive(_) | ProcessFrame::Json(_) | ProcessFrame::Data(_) => {
        // improvement: support pts to terminate
        0
      }
    }
  }
}
