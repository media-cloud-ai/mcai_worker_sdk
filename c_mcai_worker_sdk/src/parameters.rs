use schemars::{
  gen::SchemaGenerator,
  schema::{InstanceType, ObjectValidation, Schema, SchemaObject},
  JsonSchema,
};

use crate::get_c_string;
use crate::utils::get_worker_parameters;
use crate::worker::CWorkerParameter;
use mcai_worker_sdk::prelude::*;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::ffi::CStr;
use std::os::raw::c_char;

#[derive(Deserialize, Debug, Clone)]
pub struct CWorkerParameters {
  #[serde(flatten)]
  pub parameters: HashMap<String, Value>,
}

fn get_instance_type_from_parameter_type(parameter_type: &WorkerParameterType) -> InstanceType {
  match parameter_type {
    WorkerParameterType::String => InstanceType::String,
    WorkerParameterType::ArrayOfStrings => InstanceType::Array,
    WorkerParameterType::Boolean => InstanceType::Boolean,
    WorkerParameterType::Credential => InstanceType::String,
    WorkerParameterType::Integer => InstanceType::Integer,
    WorkerParameterType::Requirements => InstanceType::Array,
  }
}

impl JsonSchema for CWorkerParameters {
  fn schema_name() -> String {
    "CWorkerParameters".to_string()
  }

  fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
    let parameters = get_worker_parameters();

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
        properties: schema_parameters,
        ..Default::default()
      })),
      ..Default::default()
    };

    schema.into()
  }
}

fn get_parameter_type_from_c_str(c_str: &CStr) -> WorkerParameterType {
  match c_str.to_str() {
    Ok(c_str) => {
      // keep string quotes in string to json deserializer
      let json_string = format!("{:?}", c_str);
      match serde_json::from_str(&json_string) {
        Ok(parameter_type) => parameter_type,
        Err(msg) => panic!(
          "unable to deserialize worker parameter type {:?}: {:?}",
          json_string, msg
        ),
      }
    }
    Err(msg) => panic!("unable to parse worker parameter type: {:?}", msg),
  }
}

pub unsafe fn get_parameter_from_worker_parameter(
  worker_parameter: &CWorkerParameter,
) -> WorkerParameter {
  let identifier = get_c_string!(worker_parameter.identifier);
  let label = get_c_string!(worker_parameter.label);
  let kind_list: &[*const c_char] =
    std::slice::from_raw_parts(worker_parameter.kind, worker_parameter.kind_size);

  let parameter_types = kind_list
    .iter()
    .map(|kind| get_parameter_type_from_c_str(CStr::from_ptr(*kind)))
    .collect();

  let required = worker_parameter.required > 0;

  WorkerParameter {
    identifier,
    label,
    kind: parameter_types,
    required,
  }
}

#[test]
pub fn test_get_instance_type_from_parameter() {
  assert_eq!(
    InstanceType::String,
    get_instance_type_from_parameter_type(&WorkerParameterType::String)
  );
  assert_eq!(
    InstanceType::Array,
    get_instance_type_from_parameter_type(&WorkerParameterType::ArrayOfStrings)
  );
  assert_eq!(
    InstanceType::Boolean,
    get_instance_type_from_parameter_type(&WorkerParameterType::Boolean)
  );
  assert_eq!(
    InstanceType::String,
    get_instance_type_from_parameter_type(&WorkerParameterType::Credential)
  );
  assert_eq!(
    InstanceType::Integer,
    get_instance_type_from_parameter_type(&WorkerParameterType::Integer)
  );
  assert_eq!(
    InstanceType::Array,
    get_instance_type_from_parameter_type(&WorkerParameterType::Requirements)
  );
}
