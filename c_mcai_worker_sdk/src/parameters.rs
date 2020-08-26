use schemars::{
  gen::SchemaGenerator,
  schema::{InstanceType, ObjectValidation, Schema, SchemaObject},
  JsonSchema,
};

use crate::worker::get_worker_parameters;
use mcai_worker_sdk::worker::ParameterType;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

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
