use super::{AudioFormat, RegionOfInterest, Scaling, VideoFormat};
#[cfg(all(feature = "media", feature = "python"))]
use dict_derive::{FromPyObject, IntoPyObject};
use schemars::JsonSchema;
use stainless_ffmpeg::{
  order::{Filter, ParameterValue},
  video_decoder::VideoDecoder,
};
use std::collections::HashMap;

pub trait FilterParameters {
  fn get_filter_parameters(&self) -> HashMap<String, String>;
}

#[cfg(feature = "media")]
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct GenericFilter {
  pub name: String,
  pub label: Option<String>,
  pub parameters: HashMap<String, String>,
}

impl GenericFilter {
  pub fn as_filter(&self) -> Result<Filter, String> {
    let parameters = self
      .parameters
      .iter()
      .map(|(key, value)| (key.clone(), ParameterValue::String(value.clone())))
      .collect();
    Ok(Filter {
      name: self.name.clone(),
      label: self.label.clone(),
      parameters,
      inputs: None,
      outputs: None,
    })
  }
}

#[cfg(feature = "media")]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum AudioFilter {
  Format(AudioFormat),
  Generic(GenericFilter),
}

impl AudioFilter {
  pub fn as_generic_filter(&self) -> Result<GenericFilter, String> {
    match self {
      AudioFilter::Format(audio_format) => Ok(GenericFilter {
        name: "aformat".to_string(),
        label: Some("aformat_filter".to_string()),
        parameters: audio_format.get_filter_parameters(),
      }),
      AudioFilter::Generic(generic_filter) => Ok(generic_filter.clone()),
    }
  }
}

#[cfg(feature = "media")]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum VideoFilter {
  Crop(RegionOfInterest),
  Resize(Scaling),
  Format(VideoFormat),
  Generic(GenericFilter),
}

impl VideoFilter {
  pub fn as_generic_filter(&self, video_decoder: &VideoDecoder) -> Result<GenericFilter, String> {
    match self {
      VideoFilter::Crop(region_of_interest) => {
        let image_width = video_decoder.get_width() as u32;
        let image_height = video_decoder.get_height() as u32;
        let coordinates = region_of_interest.get_crop_coordinates(image_width, image_height)?;
        Ok(GenericFilter {
          name: "crop".to_string(),
          label: Some("crop_filter".to_string()),
          parameters: coordinates.get_filter_parameters(),
        })
      }
      VideoFilter::Resize(scaling) => Ok(GenericFilter {
        name: "scale".to_string(),
        label: Some("scale_filter".to_string()),
        parameters: scaling.get_filter_parameters(),
      }),
      VideoFilter::Format(video_format) => {
        let mut parameters = HashMap::<String, String>::new();
        parameters.insert("pix_fmts".to_string(), video_format.pixel_formats.clone());

        Ok(GenericFilter {
          name: "format".to_string(),
          label: Some("format_filter".to_string()),
          parameters,
        })
      }
      VideoFilter::Generic(generic_filter) => Ok(generic_filter.clone()),
    }
  }
}
