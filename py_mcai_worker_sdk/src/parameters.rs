use std::collections::{BTreeMap, HashMap};

use pyo3::{types::*, Python};
use schemars::{
  gen::SchemaGenerator,
  schema::{InstanceType, ObjectValidation, Schema, SchemaObject},
  JsonSchema,
};
use serde_json::Value;

use mcai_worker_sdk::{worker::ParameterType, MessageError, Result};

use crate::{helpers::py_err_to_string, PythonWorkerEvent};

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

    let schema_parameters: BTreeMap<String, Schema> = parameters
      .iter()
      .map(|parameter| {
        let parameter_type = &parameter.kind[0];

        let instance_type = if parameter.required {
          get_instance_type_from_parameter_type(parameter_type).into()
        } else {
          vec![
            get_instance_type_from_parameter_type(parameter_type),
            InstanceType::Null,
          ]
          .into()
        };

        let instance_type = Some(instance_type);

        let object = SchemaObject {
          instance_type,
          ..Default::default()
        };

        (parameter.identifier.clone(), object.into())
      })
      .collect();

    let schema = SchemaObject {
      instance_type: Some(InstanceType::Object.into()),
      object: Some(Box::new(ObjectValidation {
        properties: schema_parameters,
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
    ParameterType::Requirements => InstanceType::Object,
  }
}

pub fn build_parameters(parameters: PythonWorkerParameters, py: Python) -> Result<&PyDict> {
  let list_of_parameters = PyDict::new(py);
  for (key, value) in parameters.parameters {
    match value {
      Value::String(string) => {
        list_of_parameters.set_item(key, string).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot set item to parameters: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Null => {}
      Value::Bool(boolean) => {
        list_of_parameters.set_item(key, boolean).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot set item to parameters: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Number(number) => {
        list_of_parameters
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
        list_of_parameters.set_item(key, list).map_err(|e| {
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
        array.append(string).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot append item to array: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Null => {}
      Value::Bool(boolean) => {
        array.append(boolean).map_err(|e| {
          MessageError::ParameterValueError(format!(
            "Cannot append item to array: {}",
            py_err_to_string(py, e)
          ))
        })?;
      }
      Value::Number(number) => {
        array.append(number.as_u64()).map_err(|e| {
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

#[test]
pub fn test_get_instance_type_from_parameter() {
  assert_eq!(
    InstanceType::String,
    get_instance_type_from_parameter_type(&ParameterType::String)
  );
  assert_eq!(
    InstanceType::Array,
    get_instance_type_from_parameter_type(&ParameterType::ArrayOfStrings)
  );
  assert_eq!(
    InstanceType::Boolean,
    get_instance_type_from_parameter_type(&ParameterType::Boolean)
  );
  assert_eq!(
    InstanceType::String,
    get_instance_type_from_parameter_type(&ParameterType::Credential)
  );
  assert_eq!(
    InstanceType::Integer,
    get_instance_type_from_parameter_type(&ParameterType::Integer)
  );
  assert_eq!(
    InstanceType::Object,
    get_instance_type_from_parameter_type(&ParameterType::Requirements)
  );
}

#[test]
pub fn test_build_parameters() {
  let mut parameters = HashMap::<String, Value>::new();
  parameters.insert(
    "string_parameter".to_string(),
    Value::String("string_value".to_string()),
  );
  parameters.insert("null_parameter".to_string(), Value::Null);
  parameters.insert("boolean_parameter".to_string(), Value::Bool(true));
  parameters.insert(
    "number_parameter".to_string(),
    Value::Number(serde_json::Number::from(123)),
  );
  parameters.insert(
    "array_of_string_parameter".to_string(),
    Value::Array(vec![Value::String("string_value".to_string())]),
  );
  parameters.insert(
    "array_of_null_parameter".to_string(),
    Value::Array(vec![Value::Null]),
  );
  parameters.insert(
    "array_of_bool_parameter".to_string(),
    Value::Array(vec![Value::Bool(true)]),
  );
  parameters.insert(
    "array_of_number_parameter".to_string(),
    Value::Array(vec![Value::Number(serde_json::Number::from(123))]),
  );
  let worker_parameters = PythonWorkerParameters { parameters };

  let gil = Python::acquire_gil();
  let py = gil.python();

  let result = build_parameters(worker_parameters, py);
  assert!(result.is_ok());
  let py_parameters = result.unwrap();
  assert!(py_parameters.get_item("string_parameter").is_some());
  assert!(py_parameters.get_item("boolean_parameter").is_some());
  assert!(py_parameters.get_item("number_parameter").is_some());
  assert!(py_parameters
    .get_item("array_of_string_parameter")
    .is_some());
  assert!(py_parameters.get_item("array_of_null_parameter").is_some());
  assert!(py_parameters.get_item("array_of_bool_parameter").is_some());
  assert!(py_parameters
    .get_item("array_of_number_parameter")
    .is_some());
  assert!(py_parameters.get_item("null_parameter").is_none());
}

#[test]
pub fn test_build_parameters_with_object_value() {
  let mut parameters = HashMap::<String, Value>::new();
  let parameter_key = "object_parameter".to_string();
  parameters.insert(
    parameter_key.clone(),
    Value::Object(serde_json::Map::<String, Value>::new()),
  );
  let worker_parameters = PythonWorkerParameters { parameters };

  let gil = Python::acquire_gil();
  let py = gil.python();

  let expected_error =
    MessageError::ParameterValueError("Unsupported parameter object value: {}".to_string());
  let result = build_parameters(worker_parameters, py);
  assert!(result.is_err());
  assert_eq!(expected_error, result.unwrap_err());
}

#[test]
pub fn test_build_parameters_with_array_of_array_value() {
  let mut parameters = HashMap::<String, Value>::new();
  let parameter_key = "array_of_array_parameter".to_string();
  parameters.insert(
    parameter_key.clone(),
    Value::Array(vec![Value::Array(vec![Value::Bool(true)])]),
  );
  let worker_parameters = PythonWorkerParameters { parameters };

  let gil = Python::acquire_gil();
  let py = gil.python();

  let expected_error = MessageError::ParameterValueError(
    "Unsupported parameter array of array value: Array([Bool(true)])".to_string(),
  );
  let result = build_parameters(worker_parameters, py);
  assert!(result.is_err());
  assert_eq!(expected_error, result.unwrap_err());
}

#[test]
pub fn test_build_parameters_with_array_of_object_value() {
  let mut parameters = HashMap::<String, Value>::new();
  let parameter_key = "array_of_object_parameter".to_string();
  parameters.insert(
    parameter_key.clone(),
    Value::Array(vec![Value::Object(serde_json::Map::<String, Value>::new())]),
  );
  let worker_parameters = PythonWorkerParameters { parameters };

  let gil = Python::acquire_gil();
  let py = gil.python();

  let expected_error = MessageError::ParameterValueError(
    "Unsupported parameter array of object value: Object({})".to_string(),
  );
  let result = build_parameters(worker_parameters, py);
  assert!(result.is_err());
  assert_eq!(expected_error, result.unwrap_err());
}
