use std::collections::{BTreeMap, HashMap};

use pyo3::{types::*, Python};
use schemars::{
  gen::SchemaGenerator,
  schema::{InstanceType, ObjectValidation, Schema, SchemaObject},
  JsonSchema,
};
use serde_json::Value;

use mcai_worker_sdk::prelude::*;

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

fn get_instance_type_from_parameter_type(parameter_type: &WorkerParameterType) -> InstanceType {
  match parameter_type {
    WorkerParameterType::String => InstanceType::String,
    WorkerParameterType::ArrayOfStrings => InstanceType::Array,
    WorkerParameterType::Boolean => InstanceType::Boolean,
    WorkerParameterType::Credential => InstanceType::String,
    WorkerParameterType::Integer => InstanceType::Integer,
    WorkerParameterType::Requirements => InstanceType::Object,
  }
}

pub fn build_parameters(parameters: PythonWorkerParameters, py: Python) -> Result<&PyDict> {
  let list_of_parameters = PyDict::new(py);
  for (key, value) in parameters.parameters {
    serde_json_to_pyo3_value(&key, &value, list_of_parameters, py).map_err(|e| {
      MessageError::ParameterValueError(format!(
        "Cannot build parameters: {}",
        py_err_to_string(py, e)
      ))
    })?;
  }
  Ok(list_of_parameters)
}

fn serde_json_to_pyo3_value(
  key: &str,
  value: &Value,
  result: &PyDict,
  py: Python,
) -> pyo3::PyResult<()> {
  match value {
    Value::Null => {}
    Value::Bool(boolean) => result.set_item(key, boolean)?,
    Value::Number(number) => result.set_item(key, number.as_u64())?,
    Value::String(content) => result.set_item(key, content)?,
    Value::Array(values) => {
      let list = PyList::empty(py);
      for value in values {
        add_value_to_py_list(&value, list, py)?;
      }

      result.set_item(key, list)?;
    }
    Value::Object(map) => {
      let object = PyDict::new(py);
      for (key, value) in map.iter() {
        serde_json_to_pyo3_value(key, value, object, py)?;
      }
      result.set_item(key, object)?;
    }
  }
  Ok(())
}

fn add_value_to_py_list(value: &Value, list: &PyList, py: Python) -> pyo3::PyResult<()> {
  match value {
    Value::String(string) => {
      list.append(string)?;
    }
    Value::Null => {}
    Value::Bool(boolean) => {
      list.append(boolean)?;
    }
    Value::Number(number) => {
      list.append(number.as_u64())?;
    }
    Value::Array(values) => {
      let sub_list = PyList::empty(py);
      for value in values {
        add_value_to_py_list(&value, sub_list, py)?;
      }
      list.append(sub_list)?;
    }
    Value::Object(map) => {
      let object = PyDict::new(py);
      for (key, value) in map.iter() {
        serde_json_to_pyo3_value(key, value, object, py)?;
      }
      list.append(object)?;
    }
  }
  Ok(())
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

  let result = build_parameters(worker_parameters, py);
  assert!(result.is_ok());

  let reference = PyDict::new(py);
  reference.set_item(parameter_key, PyDict::new(py)).unwrap();
  assert_eq!(
    core::cmp::Ordering::Equal,
    result.unwrap().compare(reference).unwrap()
  );
}

#[test]
pub fn test_build_parameters_for_requirements() {
  use serde_json::json;

  let mut parameters = HashMap::<String, Value>::new();
  let parameter_key = "requirements".to_string();

  let value = json!({
    "paths": []
  });

  parameters.insert(parameter_key.clone(), value);
  let worker_parameters = PythonWorkerParameters { parameters };

  let gil = Python::acquire_gil();
  let py = gil.python();

  let result = build_parameters(worker_parameters, py);
  assert!(result.is_ok());

  let reference = PyDict::new(py);
  let requirement_content = PyDict::new(py);
  requirement_content
    .set_item("paths", PyList::empty(py))
    .unwrap();
  reference
    .set_item(parameter_key, requirement_content)
    .unwrap();
  assert_eq!(
    core::cmp::Ordering::Equal,
    result.unwrap().compare(reference).unwrap()
  );
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

  let result = build_parameters(worker_parameters, py);

  let reference = PyDict::new(py);
  let content = PyList::empty(py);
  let sub_content = PyList::empty(py);
  sub_content.append(PyBool::new(py, true)).unwrap();
  content.append(sub_content).unwrap();
  reference.set_item(parameter_key, content).unwrap();
  assert_eq!(
    core::cmp::Ordering::Equal,
    result.unwrap().compare(reference).unwrap()
  );
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

  let result = build_parameters(worker_parameters, py);

  let reference = PyDict::new(py);
  let content = PyList::empty(py);
  content.append(PyDict::new(py)).unwrap();
  reference.set_item(parameter_key, content).unwrap();
  assert_eq!(
    core::cmp::Ordering::Equal,
    result.unwrap().compare(reference).unwrap()
  );
}
