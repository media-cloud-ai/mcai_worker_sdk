use crate::parameter::{Parameter, ParameterValue, ParameterValueError};
use serde::de::DeserializeOwned;
use std::collections::HashMap;

pub trait ParametersContainer {
  fn get_parameters(&self) -> &Vec<Parameter>;

  fn get_parameter<T: DeserializeOwned>(&self, key: &str) -> Result<T, ParameterValueError>
  where
    T: ParameterValue,
  {
    for parameter in self.get_parameters() {
      if parameter.id == key && T::get_type_as_string() == parameter.kind {
        if let Some(value) = parameter.value.clone() {
          return T::parse_value(value, &parameter.store);
        } else if let Some(default) = parameter.default.clone() {
          return T::parse_value(default, &parameter.store);
        }
      }
    }
    Err(ParameterValueError::new(&format!(
      "Could not find any parameter for key '{}'",
      key
    )))
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
