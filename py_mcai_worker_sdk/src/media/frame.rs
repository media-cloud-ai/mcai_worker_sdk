use std::os::raw::c_uchar;

use mcai_worker_sdk::{MessageError, Result};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};

#[pyclass(unsendable)] // Warning: this could introduce thread unsafe behaviour
#[derive(Debug, PartialEq)]
pub struct Frame {
  #[pyo3(get)]
  pub name: Option<String>,
  #[pyo3(get)]
  pub index: usize,
  pub data: [*mut c_uchar; 8],
  #[pyo3(get)]
  pub line_size: [i32; 8],
  #[pyo3(get)]
  pub nb_samples: i32,
  #[pyo3(get)]
  pub format: i32,
  #[pyo3(get)]
  pub key_frame: i32,
  #[pyo3(get)]
  pub pts: i64,
  #[pyo3(get)]
  pub coded_picture_number: i32,
  #[pyo3(get)]
  pub display_picture_number: i32,
  #[pyo3(get)]
  pub interlaced_frame: i32,
  #[pyo3(get)]
  pub top_field_first: i32,
  #[pyo3(get)]
  pub sample_rate: i32,
  #[pyo3(get)]
  pub channels: i32,
  #[pyo3(get)]
  pub pkt_size: i32,
  #[pyo3(get)]
  pub width: i32,
  #[pyo3(get)]
  pub height: i32,
}

#[pymethods]
impl Frame {
  #[getter]
  fn get_data<'p>(&self, py: Python<'p>) -> PyResult<&'p PyList> {
    let data = PyList::empty(py);
    for plane_index in 0..self.data.len() {
      unsafe {
        data.append(PyBytes::from_ptr(
          py,
          self.data[plane_index],
          (self.line_size[plane_index] * self.height) as usize,
        ))?;
      }
    }
    Ok(data)
  }
}

impl Frame {
  pub fn from(frame: &mcai_worker_sdk::Frame) -> Result<Frame> {
    if frame.frame.is_null() {
      return Err(MessageError::RuntimeError(
        "Cannot initialize frame struct from null AVFrame".to_string(),
      ));
    }

    let av_frame = unsafe { *frame.frame };

    // TODO complete frame struct

    Ok(Frame {
      name: frame.name.clone(),
      index: frame.index,
      data: av_frame.data,
      line_size: av_frame.linesize,
      nb_samples: av_frame.nb_samples,
      format: av_frame.format,
      key_frame: av_frame.key_frame,
      pts: av_frame.pts,
      coded_picture_number: av_frame.coded_picture_number,
      display_picture_number: av_frame.display_picture_number,
      interlaced_frame: av_frame.interlaced_frame,
      top_field_first: av_frame.top_field_first,
      sample_rate: av_frame.sample_rate,
      channels: av_frame.channels,
      pkt_size: av_frame.pkt_size,
      width: av_frame.width,
      height: av_frame.height,
    })
  }
}

#[test]
pub fn test_frame_from_null_avframe() {
  let sdk_frame = mcai_worker_sdk::Frame {
    name: None,
    frame: std::ptr::null_mut(),
    index: 0,
  };

  let frame = Frame::from(&sdk_frame);
  assert!(frame.is_err());
  assert_eq!(
    MessageError::RuntimeError("Cannot initialize frame struct from null AVFrame".to_string()),
    frame.unwrap_err()
  );
}
