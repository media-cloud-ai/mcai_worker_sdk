#[cfg(all(feature = "media", feature = "python"))]
use dict_derive::{FromPyObject, IntoPyObject};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::message::media::filters::FilterParameters;

#[cfg(feature = "media")]
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[cfg_attr(feature = "python", derive(FromPyObject, IntoPyObject))]
pub struct AudioFormat {
  pub sample_rates: Vec<usize>,
  pub channel_layouts: Vec<String>,
  pub sample_formats: Vec<String>,
}

impl FilterParameters for AudioFormat {
  fn get_filter_parameters(&self) -> HashMap<String, String> {
    let mut parameters = HashMap::new();

    if !self.sample_rates.is_empty() {
      let sample_rates = self
        .sample_rates
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>()
        .join("|");

      parameters.insert("sample_rates".to_string(), sample_rates);
    }
    if !self.channel_layouts.is_empty() {
      parameters.insert(
        "channel_layouts".to_string(),
        self.channel_layouts.join("|"),
      );
    }
    if !self.sample_formats.is_empty() {
      parameters.insert("sample_fmts".to_string(), self.sample_formats.join("|"));
    }

    parameters
  }
}
