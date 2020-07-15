use pyo3::prelude::*;
use std::ffi::CString;
use std::os::raw::c_uchar;

#[pyclass]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Frame {
  #[pyo3(get)]
  pub name: Option<String>,
  #[pyo3(get)]
  pub index: usize,
  pub data: [Vec<u8>; 8],
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
  fn get_data(&self) -> PyResult<Vec<Vec<u8>>> {
    let mut result = vec![];
    for i in 0..self.data.len() {
      result.push(self.data[i].clone())
    }
    Ok(result)
  }
}

impl Frame {
  pub fn from(frame: &mcai_worker_sdk::Frame) -> Frame {
    let av_frame = unsafe { *frame.frame };

    let av_frame_data: [*mut c_uchar; 8] = av_frame.data;
    let mut frame_data: [Vec<u8>; 8] = Default::default();
    unsafe {
      for i in 0..8 {
        let av_frame_data_plan = av_frame_data[i] as *mut i8;
        if av_frame_data_plan == std::ptr::null_mut() {
          continue;
        }
        frame_data[i] = CString::from_raw(av_frame_data_plan).into();
      }
    }

    // TODO complete frame struct

    Frame {
      name: frame.name.clone(),
      index: frame.index,
      data: frame_data,
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
    }
  }

  pub fn display(&self) -> String {
    format!("name: {:?}, index: {:?}, line_size: {:?}, nb_samples: {:?}, format: {:?}, key_frame: {:?}, pts: {:?}, coded_picture_number: {:?}, display_picture_number: {:?}, interlaced_frame: {:?}, top_field_first: {:?}, sample_rate: {:?}, channels: {:?}, pkt_size: {:?}",
            self.name,
            self.index,
            self.line_size,
            self.nb_samples,
            self.format,
            self.key_frame,
            self.pts,
            self.coded_picture_number,
            self.display_picture_number,
            self.interlaced_frame,
            self.top_field_first,
            self.sample_rate,
            self.channels,
            self.pkt_size)
  }
}
