use crate::parameter::media_segment::MediaSegment;
use crate::parameter::{credential::Credential, Parameter};
use std::collections::HashMap;

pub trait ParametersContainer {
  fn get_parameters(&self) -> &Vec<Parameter>;
  fn get_boolean_parameter(&self, key: &str) -> Option<bool> {
    for param in self.get_parameters() {
      if let Parameter::BooleanParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(*v);
          } else {
            return *default;
          }
        }
      }
    }
    None
  }

  fn get_credential_parameter(&self, key: &str) -> Option<Credential> {
    for param in self.get_parameters() {
      if let Parameter::CredentialParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(Credential { key: v.to_string() });
          } else {
            return default.clone().map(|key| Credential { key });
          }
        }
      }
    }
    None
  }

  fn get_integer_parameter(&self, key: &str) -> Option<i64> {
    for param in self.get_parameters() {
      if let Parameter::IntegerParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(*v);
          } else {
            return *default;
          }
        }
      }
    }
    None
  }

  fn get_string_parameter(&self, key: &str) -> Option<String> {
    for param in self.get_parameters() {
      if let Parameter::StringParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.to_string());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  fn get_array_of_strings_parameter(&self, key: &str) -> Option<Vec<String>> {
    for param in self.get_parameters() {
      if let Parameter::ArrayOfStringsParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.clone());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  fn get_array_of_media_segments_parameter(&self, key: &str) -> Option<Vec<MediaSegment>> {
    for param in self.get_parameters() {
      if let Parameter::ArrayOfMediaSegmentsParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.clone());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  fn get_json_parameter(&self, key: &str) -> Option<String> {
    for param in self.get_parameters() {
      if let Parameter::JsonParam { id, default, value } = param {
        if id == key {
          if let Some(ref v) = value {
            return Some(v.to_string());
          } else {
            return default.clone();
          }
        }
      }
    }
    None
  }

  fn get_parameters_as_map(&self) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for param in self.get_parameters() {
      if param.has_value_or_default() {
        map.insert(param.get_id(), param.to_string());
      }
    }
    map
  }
}
