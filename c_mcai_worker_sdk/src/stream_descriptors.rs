use mcai_worker_sdk::GenericFilter;
use std::convert::TryFrom;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamType {
  Video,
  Audio,
  Data,
}

impl TryFrom<u8> for StreamType {
  type Error = String;

  fn try_from(value: u8) -> Result<Self, Self::Error> {
    if value == (StreamType::Video as u8) {
      return Ok(StreamType::Video);
    }
    if value == (StreamType::Audio as u8) {
      return Ok(StreamType::Audio);
    }
    if value == (StreamType::Data as u8) {
      return Ok(StreamType::Data);
    }
    Err(format!(
      "Could not find {} value into StreamType enum.",
      value
    ))
  }
}

#[test]
pub fn test_stream_type_try_from() {
  assert_eq!(StreamType::Video, StreamType::try_from(0).unwrap());
  assert_eq!(StreamType::Audio, StreamType::try_from(1).unwrap());
  assert_eq!(StreamType::Data, StreamType::try_from(2).unwrap());
  assert_eq!(
    "Could not find 3 value into StreamType enum.".to_string(),
    StreamType::try_from(3).unwrap_err()
  );
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct CStreamDescriptor {
  pub index: u32,
  pub stream_type: StreamType,
  pub filters: Vec<GenericFilter>,
}
