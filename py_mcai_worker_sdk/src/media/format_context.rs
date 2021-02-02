use pyo3::prelude::*;
use std::{
  collections::{BTreeMap, HashMap},
  sync::{Arc, Mutex},
};

#[pyclass]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct FormatContext {
  #[pyo3(get)]
  pub format_name: String,
  #[pyo3(get)]
  pub format_long_name: String,
  #[pyo3(get)]
  pub program_count: u32,
  #[pyo3(get)]
  pub start_time: Option<f32>,
  #[pyo3(get)]
  pub duration: Option<f64>,
  #[pyo3(get)]
  pub bit_rate: Option<i64>,
  #[pyo3(get)]
  pub packet_size: u32,
  #[pyo3(get)]
  pub nb_streams: u32,
  #[pyo3(get)]
  pub metadata: BTreeMap<String, String>,
  #[pyo3(get)]
  pub streams: Vec<StreamDescriptor>,
}

impl FormatContext {
  pub fn from(format_context: Arc<Mutex<mcai_worker_sdk::prelude::FormatContext>>) -> FormatContext {
    let context = format_context.lock().unwrap();

    let format_name = context.get_format_name();
    let format_long_name = context.get_format_long_name();

    let program_count = context.get_program_count();
    let start_time = context.get_start_time();
    let duration = context.get_duration();

    let bit_rate = context.get_bit_rate();
    let packet_size = context.get_packet_size();
    let nb_streams = context.get_nb_streams();

    let metadata = context.get_metadata();
    let mut streams = vec![];

    for stream_index in 0..context.get_nb_streams() {
      let stream = context.get_stream(stream_index as isize);

      let stream_descriptor = unsafe {
        StreamDescriptor {
          index: (*stream).id as u32,
          start_time,
          duration: duration.map(|value| value as f32),
          stream_metadata: Default::default(),
          nb_frames: (*stream).nb_frames as u64,
          avg_frame_rate: (*stream).avg_frame_rate.num as f32 / (*stream).avg_frame_rate.den as f32,
          r_frame_rate: (*stream).r_frame_rate.num as f32 / (*stream).r_frame_rate.den as f32,
          kind: format!("{:?}", (*(*stream).codec).codec_type),
          width: (*(*stream).codec).width as u32,
          height: (*(*stream).codec).height as u32,
          channels: (*(*stream).codec).channels as u32,
          sample_rate: (*(*stream).codec).sample_rate as u32,
        }
      };
      streams.push(stream_descriptor);
    }

    // TODO complete format context struct

    FormatContext {
      format_name,
      format_long_name,
      program_count,
      start_time,
      duration,
      bit_rate,
      packet_size,
      nb_streams,
      metadata,
      streams,
    }
  }
}

#[pyclass]
#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct StreamDescriptor {
  #[pyo3(get)]
  index: u32,
  #[pyo3(get)]
  nb_frames: u64,
  #[pyo3(get)]
  avg_frame_rate: f32,
  #[pyo3(get)]
  r_frame_rate: f32,
  #[pyo3(get)]
  kind: String,
  #[pyo3(get)]
  width: u32,
  #[pyo3(get)]
  height: u32,
  #[pyo3(get)]
  channels: u32,
  #[pyo3(get)]
  sample_rate: u32,
  #[pyo3(get)]
  start_time: Option<f32>,
  #[pyo3(get)]
  duration: Option<f32>,
  #[pyo3(get)]
  stream_metadata: HashMap<String, String>,
}
