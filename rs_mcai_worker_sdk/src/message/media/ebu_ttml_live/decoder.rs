use std::collections::VecDeque;
use std::mem::forget;

use stainless_ffmpeg::packet::Packet;

use crate::EbuTtmlLive;

#[derive(Default)]
pub struct EbuTtmlLiveDecoder {
  buffer: VecDeque<String>,
}

impl EbuTtmlLiveDecoder {
  pub fn new() -> Self {
    EbuTtmlLiveDecoder {
      buffer: VecDeque::new(),
    }
  }
  // &self,
  pub fn decode(&mut self, packet: &Packet) -> Result<Option<EbuTtmlLive>, String> {
    let data_size = unsafe { (*packet.packet).size as usize };
    let data = unsafe { (*packet.packet).data as *mut u8 };
    log::debug!("Decoding {} bytes EBU TTML live content", data_size);

    let ttml_content = unsafe { String::from_raw_parts(data, data_size, data_size) };
    log::trace!("Try decoding: {}", ttml_content);

    let ebu_ttml_live_content = self.decode_content(&ttml_content)?;

    forget(ttml_content);

    Ok(ebu_ttml_live_content)
  }

  pub fn decode_content(&mut self, ttml_content: &str) -> Result<Option<EbuTtmlLive>, String> {
    let ebu_ttml_live_content =
      if ttml_content.starts_with("<?xml version") && ttml_content.ends_with("tt>") {
        Some(yaserde::de::from_str::<EbuTtmlLive>(ttml_content)?)
      } else if ttml_content.starts_with("<?xml version") {
        log::debug!(
          "Add incomplete TTML content to buffer (buffer size: {})",
          self.buffer.len()
        );
        self.buffer.push_back(ttml_content.to_string());

        log::trace!("Incomplete TTML content added to buffer: {}", ttml_content);
        None
      } else if ttml_content.ends_with("tt>") {
        if let Some(previous_content) = self.buffer.pop_front() {
          log::debug!(
            "Get a previous TTML content from buffer to complete the new one (buffer size: {})",
            self.buffer.len()
          );
          let complete_ttml = format!("{}{}", previous_content, ttml_content);

          log::trace!("Concatenated TTML content: {}", complete_ttml);
          return self.decode_content(&complete_ttml);
        } else {
          return Err(format!("Incomplete TTML content: {}", ttml_content));
        }
      } else {
        None
      };
    Ok(ebu_ttml_live_content)
  }
}
