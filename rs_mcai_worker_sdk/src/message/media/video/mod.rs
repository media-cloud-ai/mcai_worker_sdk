mod region_of_interest;

use crate::message::media::filters::FilterParameters;
#[cfg(all(feature = "media", feature = "python"))]
use dict_derive::{FromPyObject, IntoPyObject};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use region_of_interest::RegionOfInterest;

#[cfg(feature = "media")]
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct VideoFormat {
  pub pixel_formats: String,
}

#[cfg(feature = "media")]
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct Scaling {
  pub width: Option<u32>,
  pub height: Option<u32>,
}

impl FilterParameters for Scaling {
  fn get_filter_parameters(&self) -> HashMap<String, String> {
    let width = self.width.map_or((-1).to_string(), |w| w.to_string());
    let height = self.height.map_or((-1).to_string(), |h| h.to_string());

    [("width", width), ("height", height)]
      .iter()
      .map(|(key, value)| (key.to_string(), value.clone()))
      .collect()
  }
}

#[cfg(feature = "media")]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct CropCoordinates {
  pub top: u32,
  pub left: u32,
  pub width: u32,
  pub height: u32,
}

impl FilterParameters for CropCoordinates {
  fn get_filter_parameters(&self) -> HashMap<String, String> {
    [
      ("out_w", self.width.to_string()),
      ("out_h", self.height.to_string()),
      ("x", self.left.to_string()),
      ("y", self.top.to_string()),
    ]
    .iter()
    .cloned()
    .map(|(key, value)| (key.to_string(), value))
    .collect()
  }
}

#[test]
pub fn test_get_scale_filter_parameters() {
  let scaling = Scaling {
    width: None,
    height: None,
  };
  let parameters = scaling.get_filter_parameters();
  assert_eq!(&(-1).to_string(), parameters.get("width").unwrap());
  assert_eq!(&(-1).to_string(), parameters.get("height").unwrap());

  let scaling = Scaling {
    width: Some(1234),
    height: None,
  };
  let parameters = scaling.get_filter_parameters();
  assert_eq!(&1234.to_string(), parameters.get("width").unwrap());
  assert_eq!(&(-1).to_string(), parameters.get("height").unwrap());

  let scaling = Scaling {
    width: None,
    height: Some(1234),
  };
  let parameters = scaling.get_filter_parameters();
  assert_eq!(&(-1).to_string(), parameters.get("width").unwrap());
  assert_eq!(&1234.to_string(), parameters.get("height").unwrap());

  let scaling = Scaling {
    width: Some(1234),
    height: Some(5678),
  };
  let parameters = scaling.get_filter_parameters();
  assert_eq!(&1234.to_string(), parameters.get("width").unwrap());
  assert_eq!(&5678.to_string(), parameters.get("height").unwrap());
}

#[test]
pub fn test_get_crop_filter_parameters() {
  let crop_coordinates = CropCoordinates {
    top: 147,
    left: 258,
    width: 123,
    height: 456,
  };
  let parameters = crop_coordinates.get_filter_parameters();
  assert_eq!(&147.to_string(), parameters.get("y").unwrap());
  assert_eq!(&258.to_string(), parameters.get("x").unwrap());
  assert_eq!(&123.to_string(), parameters.get("out_w").unwrap());
  assert_eq!(&456.to_string(), parameters.get("out_h").unwrap());
}
