use regex::Regex;
use std::cmp::Ordering;
use std::fmt;
use std::io::{Read, Write};
use xml::{reader, writer};
use yaserde::{YaDeserialize, YaSerialize};

#[derive(Clone, Debug, PartialEq)]
pub enum TimeExpression {
  ClockTime {
    hours: u16,
    minutes: u8,
    seconds: u8,
    frames: Frames,
  },
  OffsetTime {
    offset: f32,
    unit: TimeUnit,
  },
}

impl TimeExpression {
  pub fn new_frames(offset: f32) -> Self {
    TimeExpression::OffsetTime {
      offset,
      unit: TimeUnit::Frames
    }
  }

  pub fn to_frames(&self) -> f32 {
    let fps = 25.0;

    match self {
      TimeExpression::OffsetTime { offset, unit } => match unit {
        TimeUnit::Ticks => *offset,
        TimeUnit::Frames => *offset,
        TimeUnit::Milliseconds => *offset * fps,
        TimeUnit::Seconds => *offset * 1000.0 * fps,
        TimeUnit::Minutes => *offset * 1000.0 * 60.0 * fps,
        TimeUnit::Hours => *offset * 1000.0 * 60.0 * 60.0 * fps,
      },
      TimeExpression::ClockTime {
        hours,
        minutes,
        seconds,
        frames,
      } => {
        let f = match frames {
          Frames::Frames { value } => f32::from(*value),
          Frames::SubFrames { value } => f32::from(*value) * 0.001 * fps,
        };

        ((f32::from(*hours) * 60.0 + f32::from(*minutes)) * 60.0 + f32::from(*seconds)) * fps + f
      }
    }
  }

  pub fn to_timecode(&self) -> String {
    let fps = 25.0;

    match self {
      TimeExpression::OffsetTime { .. } => {
        let time_frames = self.to_frames();

        let frames = time_frames % fps;
        let seconds = (time_frames / fps) % 60.0;
        let minutes = (time_frames / (fps * 60.0)) % 60.0;
        let hours = time_frames / (fps * 60.0 * 60.0);

        format!("{:02}:{:02}:{:02}:{:02}", hours, minutes, seconds, frames)
      }
      TimeExpression::ClockTime {
        hours,
        minutes,
        seconds,
        frames,
      } => match frames {
        Frames::Frames { value } => {
          format!("{:02}:{:02}:{:02}:{:02}", hours, minutes, seconds, value)
        }
        Frames::SubFrames { value } => {
          format!("{:02}:{:02}:{:02}.{:02}", hours, minutes, seconds, value)
        }
      },
    }
  }
}

impl fmt::Display for TimeExpression {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      TimeExpression::ClockTime {
        hours,
        minutes,
        seconds,
        frames,
      } => {
        let frames_str = match frames {
          Frames::Frames { value } => format!(":{:02}", value),
          Frames::SubFrames { value } => format!(".{}", value),
        };
        write!(
          f,
          "{:02}:{:02}:{:02}{}",
          hours, minutes, seconds, frames_str
        )
      }
      TimeExpression::OffsetTime { offset, unit } => write!(f, "{}{}", offset, unit),
    }
  }
}

impl Ord for TimeExpression {
  fn cmp(&self, other: &TimeExpression) -> Ordering {
    let self_frames = self.to_frames();
    let other_frames = other.to_frames();

    match self_frames {
      x if x > other_frames => Ordering::Greater,
      x if x < other_frames => Ordering::Less,
      _ => Ordering::Equal,
    }
  }
}

impl PartialOrd for TimeExpression {
  fn partial_cmp(&self, other: &TimeExpression) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Eq for TimeExpression {}

impl Default for TimeExpression {
  fn default() -> TimeExpression {
    TimeExpression::OffsetTime {
      offset: 0.0,
      unit: TimeUnit::Frames,
    }
  }
}

impl YaDeserialize for TimeExpression {
  fn deserialize<R: Read>(reader: &mut yaserde::de::Deserializer<R>) -> Result<Self, String> {
    if let reader::XmlEvent::StartElement { ref name, .. } = reader.peek()?.to_owned() {
      if name.local_name.as_str() == "TimeExpression" {
        let _ = reader.next_event();
        if let reader::XmlEvent::Characters(content) = reader.peek()?.to_owned() {
          let mut value = content.to_owned();
          if content.ends_with('h') {
            value.truncate(content.len() - 1);
            if let Ok(offset) = value.parse::<f32>() {
              return Ok(TimeExpression::OffsetTime {
                offset,
                unit: TimeUnit::Hours,
              });
            }
            return Err("unable to parse number of TimeExpression".to_owned());
          }
          if content.ends_with('m') {
            value.truncate(content.len() - 1);
            if let Ok(offset) = value.parse::<f32>() {
              return Ok(TimeExpression::OffsetTime {
                offset,
                unit: TimeUnit::Minutes,
              });
            }
            return Err("unable to parse number of TimeExpression".to_owned());
          }
          if content.ends_with("ms") {
            value.truncate(content.len() - 2);
            if let Ok(offset) = value.parse::<f32>() {
              return Ok(TimeExpression::OffsetTime {
                offset,
                unit: TimeUnit::Milliseconds,
              });
            }
            return Err("unable to parse number of TimeExpression".to_owned());
          }
          if content.ends_with('s') {
            value.truncate(content.len() - 1);
            if let Ok(offset) = value.parse::<f32>() {
              return Ok(TimeExpression::OffsetTime {
                offset,
                unit: TimeUnit::Seconds,
              });
            }
            return Err("unable to parse number of TimeExpression".to_owned());
          }
          if content.ends_with('f') {
            value.truncate(content.len() - 1);
            if let Ok(offset) = value.parse::<f32>() {
              return Ok(TimeExpression::OffsetTime {
                offset,
                unit: TimeUnit::Frames,
              });
            }
            return Err("unable to parse number of TimeExpression".to_owned());
          }
          if content.ends_with('t') {
            value.truncate(content.len() - 1);
            if let Ok(offset) = value.parse::<f32>() {
              return Ok(TimeExpression::OffsetTime {
                offset,
                unit: TimeUnit::Ticks,
              });
            }
            return Err("unable to parse number of TimeExpression".to_owned());
          }

          let re_frames = Regex::new(r"^(\d+):(\d{2}):(\d{2}):(\d+)$").unwrap();
          if re_frames.is_match(&value) {
            let capture = re_frames.captures(&value).unwrap();

            return Ok(TimeExpression::ClockTime {
              hours: capture.get(1).unwrap().as_str().parse::<u16>().unwrap(),
              minutes: capture.get(2).unwrap().as_str().parse::<u8>().unwrap(),
              seconds: capture.get(3).unwrap().as_str().parse::<u8>().unwrap(),
              frames: Frames::Frames {
                value: capture.get(4).unwrap().as_str().parse::<u16>().unwrap(),
              },
            });
          }

          let re_subframes = Regex::new(r"^(\d+):(\d{2}):(\d{2}).(\d+)$").unwrap();
          if re_subframes.is_match(&value) {
            let capture = re_subframes.captures(&value).unwrap();

            return Ok(TimeExpression::ClockTime {
              hours: capture.get(1).unwrap().as_str().parse::<u16>().unwrap(),
              minutes: capture.get(2).unwrap().as_str().parse::<u8>().unwrap(),
              seconds: capture.get(3).unwrap().as_str().parse::<u8>().unwrap(),
              frames: Frames::SubFrames {
                value: capture.get(4).unwrap().as_str().parse::<u16>().unwrap(),
              },
            });
          }

          let re_frames = Regex::new(r"^(\d+):(\d{2}):(\d{2})$").unwrap();
          if re_frames.is_match(&value) {
            let capture = re_frames.captures(&value).unwrap();

            return Ok(TimeExpression::ClockTime {
              hours: capture.get(1).unwrap().as_str().parse::<u16>().unwrap(),
              minutes: capture.get(2).unwrap().as_str().parse::<u8>().unwrap(),
              seconds: capture.get(3).unwrap().as_str().parse::<u8>().unwrap(),
              frames: Frames::Frames { value: 0 },
            });
          }

          return Err(format!("unable to parse TimeExpression: {}", content));
        }
      }
      Err("unable to parse TimeExpression".to_owned())
    } else {
      Err("missing TimeExpression start element".to_owned())
    }
  }
}

impl YaSerialize for TimeExpression {
  fn serialize<W: Write>(&self, writer: &mut yaserde::ser::Serializer<W>) -> Result<(), String> {
    let content = format!("{}", self);
    let event = writer::XmlEvent::characters(&content);
    writer.write(event).map_err(|e| e.to_string())
  }

  fn serialize_attributes(
    &self,
    attributes: std::vec::Vec<xml::attribute::OwnedAttribute>,
    namespace: xml::namespace::Namespace,
  ) -> std::result::Result<
    (
      std::vec::Vec<xml::attribute::OwnedAttribute>,
      xml::namespace::Namespace,
    ),
    std::string::String,
  > {
    Ok((attributes, namespace))
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Frames {
  Frames { value: u16 },
  SubFrames { value: u16 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimeUnit {
  Hours,
  Minutes,
  Seconds,
  Milliseconds,
  Frames,
  Ticks,
}

impl fmt::Display for TimeUnit {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      TimeUnit::Hours => write!(f, "h"),
      TimeUnit::Minutes => write!(f, "m"),
      TimeUnit::Seconds => write!(f, "s"),
      TimeUnit::Milliseconds => write!(f, "ms"),
      TimeUnit::Frames => write!(f, "f"),
      TimeUnit::Ticks => write!(f, "t"),
    }
  }
}

#[test]
fn time_expressions() {
  use yaserde::de::from_str;
  use yaserde::ser::to_string;

  fn convert_and_check(src: &str) {
    let contents = format!("<TimeExpression>{}</TimeExpression>", src);
    let loaded: TimeExpression = from_str(&contents).unwrap();
    assert_eq!(to_string(&loaded), Ok(src.to_string()));
  }

  convert_and_check("5832f");
  convert_and_check("246.246s");
  convert_and_check("257257t");
  convert_and_check("00:04:09.249");
  convert_and_check("00:04:09.0");
  convert_and_check("00:04:14:06");
  convert_and_check("00:04:14:00");
}

#[test]
fn compare_time_expressions() {
  let src1 = TimeExpression::ClockTime {
    hours: 0,
    minutes: 0,
    seconds: 56,
    frames: Frames::Frames { value: 08 },
  };

  let src2 = TimeExpression::OffsetTime {
    offset: 1352.0,
    unit: TimeUnit::Frames,
  };

  let src3 = TimeExpression::ClockTime {
    hours: 0,
    minutes: 0,
    seconds: 56,
    frames: Frames::SubFrames { value: 333 },
  };

  println!("{} {}", src1.to_frames(), src2.to_frames());
  assert_eq!(src1.to_frames(), src2.to_frames());
  assert!(src1.to_frames() - src3.to_frames() < 0.1);
}
