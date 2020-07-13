use std::collections::{BTreeMap, HashMap};

use pyo3::{types::*, PyErr, Python};
use schemars::{
  gen::SchemaGenerator,
  schema::{InstanceType, ObjectValidation, Schema, SchemaObject},
  JsonSchema,
};
use serde_json::Value;

use mcai_worker_sdk::{worker::ParameterType, MessageError, Result};

use crate::PythonWorkerEvent;

#[derive(Deserialize)]
pub struct PythonWorkerParameters {
  #[serde(flatten)]
  parameters: HashMap<String, Value>,
}

impl JsonSchema for PythonWorkerParameters {
  fn schema_name() -> String {
    "PythonWorkerParameters".to_string()
  }

  fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
    let parameters = PythonWorkerEvent::get_parameters();

    let mut schema_parameters = BTreeMap::<String, Schema>::new();
    for parameter in &parameters {
      let parameter_type = &parameter.kind[0];
      let object = SchemaObject {
        instance_type: Some(if parameter.required {
          get_instance_type_from_parameter_type(parameter_type).into()
        } else {
          vec![
            get_instance_type_from_parameter_type(parameter_type),
            InstanceType::Null,
          ]
          .into()
        }),
        ..Default::default()
      };
      schema_parameters.insert(parameter.identifier.clone(), object.into());
    }

    let schema = SchemaObject {
      instance_type: Some(InstanceType::Object.into()),
      object: Some(Box::new(ObjectValidation {
        properties: schema_parameters.into(),
        ..Default::default()
      })),
      ..Default::default()
    };

    schema.into()
  }
}

fn get_instance_type_from_parameter_type(parameter_type: &ParameterType) -> InstanceType {
  match parameter_type {
    ParameterType::String => InstanceType::String,
    ParameterType::ArrayOfStrings => InstanceType::Array,
    ParameterType::Boolean => InstanceType::Boolean,
    ParameterType::Credential => InstanceType::String,
    ParameterType::Integer => InstanceType::Integer,
    ParameterType::Requirement => InstanceType::Array,
  }
}

fn py_err_to_string(py: Python, error: PyErr) -> String {
  let locals = [("error", error)].into_py_dict(py);

  py.eval("repr(error)", None, Some(locals))
    .expect("Unknown python error, unable to get the error message")
    .to_string()
}

pub fn build_parameters(parameters: PythonWorkerParameters, py: Python) -> Result<&PyDict> {
  let list_of_parameters = PyDict::new(py);
  for (key, value) in parameters.parameters {
    match value {
      Value::String(string) => {
        let _result = list_of_parameters.set_item(key, string).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot set item to parameters: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Null => {}
      Value::Bool(boolean) => {
        let _result = list_of_parameters.set_item(key, boolean).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot set item to parameters: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Number(number) => {
        let _result = list_of_parameters
          .set_item(key, number.as_u64())
          .map_err(|e| {
            MessageError::ParameterValueError(format!(
              "Cannot set item to parameters: {}",
              py_err_to_string(py, e)
            ))
          })?;
      }
      Value::Array(array) => {
        let list = get_parameters_array_values(array, py)?;
        let _result = list_of_parameters.set_item(key, list).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot set item to parameters: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Object(map) => {
        return Err(MessageError::ParameterValueError(format!(
          "Unsupported parameter object value: {:?}",
          map
        )));
      }
    }
  }
  Ok(list_of_parameters)
}

fn get_parameters_array_values(values: Vec<Value>, py: Python) -> Result<&PyList> {
  let array = PyList::empty(py);
  for value in values {
    match value {
      Value::String(string) => {
        let _result = array.append(string).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot append item to array: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Null => {}
      Value::Bool(boolean) => {
        let _result = array.append(boolean).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot append item to array: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Number(number) => {
        let _result = array.append(number.as_u64()).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot append item to array: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Array(_) => {
        return Err(MessageError::ParameterValueError(format!(
          "Unsupported parameter array of array value: {:?}",
          value
        )));
      }
      Value::Object(_) => {
        return Err(MessageError::ParameterValueError(format!(
          "Unsupported parameter array of object value: {:?}",
          value
        )));
      }
    }
  }
  Ok(array)
}
