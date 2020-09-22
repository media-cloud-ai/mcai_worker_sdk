use pyo3::prelude::*;

use mcai_worker_sdk::{Frames, TimeExpression, TimeUnit};

#[pyclass]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTtmlTimeExpression {
  #[pyo3(get, set)]
  clock_time: Option<PyTtmlClockTime>,
  #[pyo3(get, set)]
  offset_time: Option<PyTtmlOffsetTime>,
}

#[pyclass]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTtmlClockTime {
  #[pyo3(get, set)]
  hours: u16,
  #[pyo3(get, set)]
  minutes: u8,
  #[pyo3(get, set)]
  seconds: u8,
  #[pyo3(get, set)]
  frames: PyTtmlFrames,
}

#[pyclass]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTtmlFrames {
  #[pyo3(get, set)]
  value: u16,
  #[pyo3(get, set)]
  sub_frames: bool,
}

#[pyclass]
#[derive(Clone, Debug, PartialEq)]
pub struct PyTtmlOffsetTime {
  #[pyo3(get, set)]
  offset: f32,
  #[pyo3(get, set)]
  unit: String,
}

impl From<Frames> for PyTtmlFrames {
  fn from(frames: Frames) -> Self {
    match frames {
      Frames::Frames { value } => PyTtmlFrames {
        value,
        sub_frames: false,
      },
      Frames::SubFrames { value } => PyTtmlFrames {
        value,
        sub_frames: true,
      },
    }
  }
}

impl Into<Frames> for PyTtmlFrames {
  fn into(self) -> Frames {
    if self.sub_frames {
      Frames::SubFrames { value: self.value }
    } else {
      Frames::Frames { value: self.value }
    }
  }
}

#[pymethods]
impl PyTtmlTimeExpression {
  pub fn to_time_code(&self) -> String {
    let time_expression: TimeExpression = self.clone().into();
    time_expression.to_timecode()
  }
}

impl From<TimeExpression> for PyTtmlTimeExpression {
  fn from(time_expression: TimeExpression) -> Self {
    match time_expression {
      TimeExpression::OffsetTime { offset, unit } => {
        let offset = PyTtmlOffsetTime {
          offset,
          unit: unit.to_string(),
        };
        PyTtmlTimeExpression {
          clock_time: None,
          offset_time: Some(offset),
        }
      }
      TimeExpression::ClockTime {
        hours,
        minutes,
        seconds,
        frames,
      } => {
        let clock_time = PyTtmlClockTime {
          hours,
          minutes,
          seconds,
          frames: frames.into(),
        };

        PyTtmlTimeExpression {
          clock_time: Some(clock_time),
          offset_time: None,
        }
      }
    }
  }
}

impl Into<TimeExpression> for PyTtmlTimeExpression {
  fn into(self) -> TimeExpression {
    if let Some(clock_time) = self.clock_time {
      return TimeExpression::ClockTime {
        hours: clock_time.hours,
        minutes: clock_time.minutes,
        seconds: clock_time.seconds,
        frames: clock_time.frames.into(),
      };
    }

    if let Some(offset_time) = self.offset_time {
      let unit = match offset_time.unit.to_lowercase().as_str() {
        "t" => TimeUnit::Ticks,
        "f" => TimeUnit::Frames,
        "ms" => TimeUnit::Milliseconds,
        "s" => TimeUnit::Seconds,
        "m" => TimeUnit::Minutes,
        "h" => TimeUnit::Hours,
        _ => unimplemented!(),
      };

      return TimeExpression::OffsetTime {
        offset: offset_time.offset,
        unit,
      };
    }

    unimplemented!()
  }
}
