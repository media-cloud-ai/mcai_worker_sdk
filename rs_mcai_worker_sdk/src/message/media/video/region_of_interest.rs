
#[cfg(all(feature = "media", feature = "python"))]
use dict_derive::{FromPyObject, IntoPyObject};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::message::media::video::CropCoordinates;

#[cfg(feature = "media")]
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct RegionOfInterest {
  top: Option<u32>,
  left: Option<u32>,
  right: Option<u32>,
  bottom: Option<u32>,
  width: Option<u32>,
  height: Option<u32>,
}

impl RegionOfInterest {
  pub fn get_crop_coordinates(
    &self,
    image_width: u32,
    image_height: u32,
  ) -> Result<CropCoordinates, String> {
    match self.clone() {
      RegionOfInterest {
        top: Some(top),
        left: Some(left),
        right: Some(right),
        bottom: Some(bottom),
        width: None,
        height: None,
      } => Ok(CropCoordinates {
        top,
        left,
        width: (image_width - right) - left,
        height: (image_height - bottom) - top,
      }),
      RegionOfInterest {
        top: Some(top),
        left: Some(left),
        right: None,
        bottom: None,
        width: Some(width),
        height: Some(height),
      } => Ok(CropCoordinates {
        top,
        left,
        width,
        height,
      }),
      RegionOfInterest {
        top: Some(top),
        left: Some(left),
        right: None,
        bottom: Some(bottom),
        width: Some(width),
        height: None,
      } => Ok(CropCoordinates {
        top,
        left,
        width,
        height: (image_height - bottom) - top,
      }),
      RegionOfInterest {
        top: Some(top),
        left: Some(left),
        right: Some(right),
        bottom: None,
        width: None,
        height: Some(height),
      } => Ok(CropCoordinates {
        top,
        left,
        width: (image_width - right) - left,
        height,
      }),
      RegionOfInterest {
        top: None,
        left: Some(left),
        right: None,
        bottom: Some(bottom),
        width: Some(width),
        height: Some(height),
      } => Ok(CropCoordinates {
        top: (image_height - bottom) - height,
        left,
        width,
        height,
      }),
      RegionOfInterest {
        top: Some(top),
        left: None,
        right: Some(right),
        bottom: None,
        width: Some(width),
        height: Some(height),
      } => Ok(CropCoordinates {
        top,
        left: (image_width - right) - width,
        width,
        height,
      }),
      RegionOfInterest {
        top: None,
        left: None,
        right: Some(right),
        bottom: Some(bottom),
        width: Some(width),
        height: Some(height),
      } => Ok(CropCoordinates {
        top: (image_height - bottom) - height,
        left: (image_width - right) - width,
        width,
        height,
      }),
      _ => Err(format!(
        "Cannot compute coordinates from such a region of interest: {:?}",
        self
      )),
    }
  }
}

#[test]
pub fn region_of_interest_to_coordinates_top_left_right_bottom() {
  let region_of_interest = RegionOfInterest {
    top: Some(0),
    left: Some(0),
    right: Some(200),
    bottom: Some(100),
    width: None,
    height: None,
  };

  let coordinates = region_of_interest.get_crop_coordinates(600, 400).unwrap();

  assert_eq!(0, coordinates.top);
  assert_eq!(0, coordinates.left);
  assert_eq!(400, coordinates.width);
  assert_eq!(300, coordinates.height);
}

#[test]
pub fn region_of_interest_to_coordinates_top_left_width_height() {
  let region_of_interest = RegionOfInterest {
    top: Some(0),
    left: Some(0),
    right: None,
    bottom: None,
    width: Some(200),
    height: Some(100),
  };

  let coordinates = region_of_interest.get_crop_coordinates(600, 400).unwrap();

  assert_eq!(0, coordinates.top);
  assert_eq!(0, coordinates.left);
  assert_eq!(200, coordinates.width);
  assert_eq!(100, coordinates.height);
}

#[test]
pub fn region_of_interest_to_coordinates_top_left_bottom_width() {
  let region_of_interest = RegionOfInterest {
    top: Some(0),
    left: Some(0),
    right: None,
    bottom: Some(100),
    width: Some(200),
    height: None,
  };

  let coordinates = region_of_interest.get_crop_coordinates(600, 400).unwrap();

  assert_eq!(0, coordinates.top);
  assert_eq!(0, coordinates.left);
  assert_eq!(200, coordinates.width);
  assert_eq!(300, coordinates.height);
}

#[test]
pub fn region_of_interest_to_coordinates_top_left_right_height() {
  let region_of_interest = RegionOfInterest {
    top: Some(0),
    left: Some(0),
    right: Some(200),
    bottom: None,
    width: None,
    height: Some(100),
  };

  let coordinates = region_of_interest.get_crop_coordinates(600, 400).unwrap();

  assert_eq!(0, coordinates.top);
  assert_eq!(0, coordinates.left);
  assert_eq!(400, coordinates.width);
  assert_eq!(100, coordinates.height);
}

#[test]
pub fn region_of_interest_to_coordinates_left_bottom_width_height() {
  let region_of_interest = RegionOfInterest {
    top: None,
    left: Some(0),
    right: None,
    bottom: Some(100),
    width: Some(200),
    height: Some(100),
  };

  let coordinates = region_of_interest.get_crop_coordinates(600, 400).unwrap();

  assert_eq!(200, coordinates.top);
  assert_eq!(0, coordinates.left);
  assert_eq!(200, coordinates.width);
  assert_eq!(100, coordinates.height);
}

#[test]
pub fn region_of_interest_to_coordinates_top_right_width_height() {
  let region_of_interest = RegionOfInterest {
    top: Some(0),
    left: None,
    right: Some(200),
    bottom: None,
    width: Some(200),
    height: Some(100),
  };

  let coordinates = region_of_interest.get_crop_coordinates(600, 400).unwrap();

  assert_eq!(0, coordinates.top);
  assert_eq!(200, coordinates.left);
  assert_eq!(200, coordinates.width);
  assert_eq!(100, coordinates.height);
}

#[test]
pub fn region_of_interest_to_coordinates_right_bottom_width_height() {
  let region_of_interest = RegionOfInterest {
    top: None,
    left: None,
    right: Some(200),
    bottom: Some(100),
    width: Some(200),
    height: Some(100),
  };

  let coordinates = region_of_interest.get_crop_coordinates(600, 400).unwrap();

  assert_eq!(200, coordinates.top);
  assert_eq!(200, coordinates.left);
  assert_eq!(200, coordinates.width);
  assert_eq!(100, coordinates.height);
}

#[test]
pub fn region_of_interest_to_coordinates_errors() {
  let roi = RegionOfInterest {
    top: None,
    left: None,
    right: None,
    bottom: Some(100),
    width: Some(200),
    height: Some(100),
  };
  let error = roi.get_crop_coordinates(600, 400).unwrap_err();
  let expected = format!(
    "Cannot compute coordinates from such a region of interest: {:?}",
    roi
  );
  assert_eq!(expected, error);

  let roi = RegionOfInterest {
    top: None,
    left: None,
    right: Some(100),
    bottom: None,
    width: Some(200),
    height: Some(100),
  };
  let error = roi.get_crop_coordinates(600, 400).unwrap_err();
  let expected = format!(
    "Cannot compute coordinates from such a region of interest: {:?}",
    roi
  );
  assert_eq!(expected, error);

  let roi = RegionOfInterest {
    top: None,
    left: None,
    right: Some(100),
    bottom: Some(100),
    width: None,
    height: Some(100),
  };
  let error = roi.get_crop_coordinates(600, 400).unwrap_err();
  let expected = format!(
    "Cannot compute coordinates from such a region of interest: {:?}",
    roi
  );
  assert_eq!(expected, error);

  let roi = RegionOfInterest {
    top: None,
    left: None,
    right: Some(100),
    bottom: Some(100),
    width: Some(200),
    height: None,
  };
  let error = roi.get_crop_coordinates(600, 400).unwrap_err();
  let expected = format!(
    "Cannot compute coordinates from such a region of interest: {:?}",
    roi
  );
  assert_eq!(expected, error);

  let roi = RegionOfInterest {
    top: None,
    left: None,
    right: None,
    bottom: None,
    width: None,
    height: None,
  };
  let error = roi.get_crop_coordinates(600, 400).unwrap_err();
  let expected = format!(
    "Cannot compute coordinates from such a region of interest: {:?}",
    roi
  );
  assert_eq!(expected, error);
}
