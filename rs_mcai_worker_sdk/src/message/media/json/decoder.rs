extern crate serde_json;

use super::Json;
use serde_json::Value;
use stainless_ffmpeg::packet::Packet;
use std::collections::VecDeque;
use std::mem::forget;

#[derive(Default)]
pub struct JsonDecoder {
  buffer: VecDeque<String>,
}

impl JsonDecoder {
  pub fn new() -> Self {
    JsonDecoder {
      buffer: VecDeque::new(),
    }
  }

  pub fn decode(&mut self, packet: &Packet) -> Result<Option<Json>, String> {
    let data_size = unsafe { (*packet.packet).size as usize };
    let data = unsafe { (*packet.packet).data as *mut u8 };
    log::trace!("Decoding {} bytes JSON content", data_size);

    let json_content = unsafe { String::from_raw_parts(data, data_size, data_size) };
    log::debug!("Try decoding: {}", json_content);

    let json_value = self.decode_content(&json_content)?;

    forget(json_content);

    Ok(json_value)
  }

  pub fn decode_content(&mut self, json_content: &str) -> Result<Option<Json>, String> {
    let json_value: Option<Json> = {
      let buffer_size = self.buffer.len();

      let principal_json = if json_content.contains("}{") {
        let vec: Vec<&str> = json_content.split("}{").collect();
        let first_json = format!("{}}}", vec[0]);
        let last_json = format!("{{{}", vec[1]);
        self.buffer.push_back(last_json);
        first_json
      } else {
        json_content.to_string()
      };

      let actual_json = if buffer_size > 0 {
        if let Some(previous_content) = self.buffer.pop_front() {
          log::debug!(
            "Get a previous JSON content from buffer to complete the new one (buffer size: {})",
            self.buffer.len()
          );
          let complete_json = format!("{}{}", previous_content, principal_json);
          log::debug!("Concatenated JSON content: {}", complete_json);

          complete_json
        } else {
          return Err(format!("Incorrect JSON content: {}", principal_json));
        }
      } else {
        principal_json
      };

      let result = serde_json::from_str(&actual_json);
      if result.is_err() {
        log::debug!(
          "Add incomplete JSON content to buffer (buffer size: {})",
          self.buffer.len()
        );

        self.buffer.push_back(actual_json.to_string());
        log::trace!("Incomplete JSON content added to buffer: {}", actual_json);
        return Ok(None);
      };

      let value: Value = result.unwrap();

      if !value.is_object() {
        return Err(format!("Incorrect JSON content: {}", actual_json));
      }

      Some(Json { value: Some(value) })
    };

    Ok(json_value)
  }
}
