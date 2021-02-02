use mcai_worker_sdk::prelude::*;
use std::convert::TryFrom;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamType {
  Video,
  Audio,
  Data,
}

impl TryFrom<u8> for StreamType {
  type Error = String;

  fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
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

#[derive(Clone, Debug)]
pub struct CStreamDescriptor {
  pub index: u32,
  pub stream_type: StreamType,
  pub filters: Vec<GenericFilter>,
}

impl Into<StreamDescriptor> for Box<CStreamDescriptor> {
  fn into(self) -> StreamDescriptor {
    match &self.stream_type {
      StreamType::Audio => {
        let audio_filters = self
          .filters
          .iter()
          .cloned()
          .map(AudioFilter::Generic)
          .collect();
        StreamDescriptor::new_audio(self.index as usize, audio_filters)
      }
      StreamType::Video => {
        let video_filters = self
          .filters
          .iter()
          .cloned()
          .map(VideoFilter::Generic)
          .collect();
        StreamDescriptor::new_video(self.index as usize, video_filters)
      }
      StreamType::Data => StreamDescriptor::new_data(self.index as usize),
    }
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

#[test]
pub fn test_c_stream_descriptor_box_into_stream_descriptor() {
  let index = 123;

  let stream_type = StreamType::Video;
  let c_descriptor = CStreamDescriptor {
    index,
    stream_type,
    filters: vec![],
  };
  let boxed_c_descriptor = Box::new(c_descriptor);
  let stream_descriptor = boxed_c_descriptor.into();
  let expected = StreamDescriptor::new_video(123, vec![]);

  assert_eq!(expected, stream_descriptor);

  let stream_type = StreamType::Audio;
  let c_descriptor = CStreamDescriptor {
    index,
    stream_type,
    filters: vec![],
  };
  let boxed_c_descriptor = Box::new(c_descriptor);
  let stream_descriptor = boxed_c_descriptor.into();
  let expected = StreamDescriptor::new_audio(123, vec![]);

  assert_eq!(expected, stream_descriptor);

  let stream_type = StreamType::Data;
  let c_descriptor = CStreamDescriptor {
    index,
    stream_type,
    filters: vec![],
  };
  let boxed_c_descriptor = Box::new(c_descriptor);
  let stream_descriptor = boxed_c_descriptor.into();
  let expected = StreamDescriptor::new_data(123);

  assert_eq!(expected, stream_descriptor);
}
