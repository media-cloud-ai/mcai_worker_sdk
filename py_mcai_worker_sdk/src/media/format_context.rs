use pyo3::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

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
  pub fn from(format_context: Arc<Mutex<mcai_worker_sdk::FormatContext>>) -> FormatContext {
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
    let streams = vec![];

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
  stream_type: String,
  #[pyo3(get)]
  codec_name: Option<String>,
  #[pyo3(get)]
  codec_long_name: Option<String>,
  #[pyo3(get)]
  codec_tag: Option<String>,
  #[pyo3(get)]
  start_time: Option<f32>,
  #[pyo3(get)]
  duration: Option<f32>,
  #[pyo3(get)]
  bit_rate: Option<i64>,
  #[pyo3(get)]
  stream_metadata: HashMap<String, String>,
}
