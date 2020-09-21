use schemars::{
  gen::SchemaGenerator,
  schema::{InstanceType, ObjectValidation, Schema, SchemaObject},
  JsonSchema,
};

use crate::get_c_string;
use crate::worker::{get_worker_parameters, WorkerParameter};
use mcai_worker_sdk::worker::{Parameter, ParameterType};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::ffi::CStr;
use std::os::raw::c_char;

#[derive(Deserialize, Debug, Clone)]
pub struct CWorkerParameters {
  #[serde(flatten)]
  pub parameters: HashMap<String, Value>,
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

fn get_parameter_type_from_c_str(c_str: &CStr) -> ParameterType {
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

pub unsafe fn get_parameter_from_worker_parameter(worker_parameter: &WorkerParameter) -> Parameter {
  let identifier = get_c_string!(worker_parameter.identifier);
  let label = get_c_string!(worker_parameter.label);
  let kind_list: &[*const c_char] =
    std::slice::from_raw_parts(worker_parameter.kind, worker_parameter.kind_size);
  let mut parameter_types = vec![];
  for kind in kind_list.iter() {
    parameter_types.push(get_parameter_type_from_c_str(CStr::from_ptr(*kind)));
  }
  let required = worker_parameter.required > 0;

  Parameter {
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
    InstanceType::Array,
    get_instance_type_from_parameter_type(&ParameterType::Requirement)
  );
}
