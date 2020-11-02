extern crate mcai_worker_sdk;

use mcai_worker_sdk::{parameter::MediaSegments, MessageError, ParameterValue, Requirement};
use serde_json::{Number, Value};

#[test]
fn test_parameter_value_types_as_string() {
  assert_eq!("string".to_string(), String::get_type_as_string());
  assert_eq!("integer".to_string(), i64::get_type_as_string());
  assert_eq!("float".to_string(), f64::get_type_as_string());
  assert_eq!("boolean".to_string(), bool::get_type_as_string());
  assert_eq!(
    "array_of_strings".to_string(),
    Vec::<String>::get_type_as_string()
  );
  assert_eq!(
    "requirements".to_string(),
    Requirement::get_type_as_string()
  );
  assert_eq!(
    "array_of_media_segments".to_string(),
    MediaSegments::get_type_as_string()
  );
}

#[test]
fn test_parameter_value_string() {
  let string = "string_value".to_string();
  let json_value = Value::String(string.clone());
  let result = String::parse_value(json_value, &None);
  assert_eq!(string, result.unwrap());

  let string = "string_env_var".to_string();
  let json_value = Value::String(string.clone());
  let store_value = "string_value_from_store".to_string();

  std::env::set_var(&string, &store_value);
  let result = String::parse_value(json_value.clone(), &Some("env".to_string()));
  assert_eq!(store_value, result.unwrap());
  std::env::remove_var(&string);

  let result = String::parse_value(json_value, &Some("env".to_string()));
  assert_eq!(
    MessageError::ParameterValueError("\"environment variable not found\"".to_string()),
    result.unwrap_err()
  );

  let result = String::parse_value(Value::Null, &Some("env".to_string()));
  assert_eq!(
    MessageError::ParameterValueError("Cannot handle credential type for Null".to_string()),
    result.unwrap_err()
  );

  let result = String::from_value(Value::Null);
  assert_eq!(
    MessageError::ParameterValueError(
      "Error(\"invalid type: null, expected a string\", line: 0, column: 0)".to_string()
    ),
    result.unwrap_err()
  );
}

#[test]
fn test_parameter_value_integer() {
  let value = 123;
  let json_value = Value::String(value.to_string());
  let result = i64::parse_value(json_value, &None);
  assert_eq!(value, result.unwrap());

  let json_value = Value::String("Hello".to_string());
  let result = i64::parse_value(json_value, &None);
  assert_eq!(
    MessageError::ParameterValueError("ParseIntError { kind: InvalidDigit }".to_string()),
    result.unwrap_err()
  );

  let value = 123;
  let json_value = Value::Number(Number::from(value));
  let result = i64::parse_value(json_value, &None);
  assert_eq!(value, result.unwrap());

  let value = 123.45678;
  let json_value = Value::Number(Number::from_f64(value).unwrap());
  let result = i64::parse_value(json_value, &None);
  assert_eq!(
    MessageError::ParameterValueError(
      "Cannot convert value type 'Number(123.45678)' to type i64".to_string()
    ),
    result.unwrap_err()
  );

  let value = true;
  let json_value = Value::Bool(value);
  let result = i64::parse_value(json_value, &None);
  assert_eq!(
    MessageError::ParameterValueError(
      "Cannot convert value type 'Bool(true)' to type i64".to_string()
    ),
    result.unwrap_err()
  );

  let string = "integer_env_var".to_string();
  let json_value = Value::String(string.clone());
  let store_value = 123456;

  std::env::set_var(&string, &store_value.to_string());
  let result = i64::parse_value(json_value.clone(), &Some("env".to_string()));
  assert_eq!(store_value, result.unwrap());
  std::env::remove_var(&string);

  let result = i64::parse_value(json_value, &Some("env".to_string()));
  assert_eq!(
    MessageError::ParameterValueError("\"environment variable not found\"".to_string()),
    result.unwrap_err()
  );
}

#[test]
fn test_parameter_value_float() {
  let value = 123.456;
  let json_value = Value::String(value.to_string());
  let result = f64::parse_value(json_value, &None);
  assert_eq!(value, result.unwrap());

  let json_value = Value::String("Hello".to_string());
  let result = f64::parse_value(json_value, &None);
  assert_eq!(
    MessageError::ParameterValueError("ParseFloatError { kind: Invalid }".to_string()),
    result.unwrap_err()
  );

  let value = 123;
  let json_value = Value::Number(Number::from(value));
  let result = f64::parse_value(json_value, &None);
  assert_eq!(value as f64, result.unwrap());

  let value = 123.456;
  let json_value = Value::Number(Number::from_f64(value).unwrap());
  let result = f64::parse_value(json_value, &None);
  assert_eq!(value, result.unwrap());

  let value = true;
  let json_value = Value::Bool(value);
  let result = f64::parse_value(json_value, &None);
  assert_eq!(
    MessageError::ParameterValueError(
      "Cannot convert value type 'Bool(true)' to type f64".to_string()
    ),
    result.unwrap_err()
  );

  let string = "float_env_var".to_string();
  let json_value = Value::String(string.clone());
  let store_value = 123.456;

  std::env::set_var(&string, &store_value.to_string());
  let result = f64::parse_value(json_value.clone(), &Some("env".to_string()));
  assert_eq!(store_value, result.unwrap());
  std::env::remove_var(&string);

  let result = f64::parse_value(json_value, &Some("env".to_string()));
  assert_eq!(
    MessageError::ParameterValueError("\"environment variable not found\"".to_string()),
    result.unwrap_err()
  );
}

#[test]
fn test_parameter_value_bool() {
  let value = true;
  let json_value = Value::String(value.to_string());
  let result = bool::parse_value(json_value, &None);
  assert_eq!(value, result.unwrap());

  let json_value = Value::String("Hello".to_string());
  let result = bool::parse_value(json_value, &None);
  assert_eq!(
    MessageError::ParameterValueError("ParseBoolError { _priv: () }".to_string()),
    result.unwrap_err()
  );

  let value = true;
  let json_value = Value::Bool(value);
  let result = bool::parse_value(json_value, &None);
  assert_eq!(value, result.unwrap());

  let value = 123;
  let json_value = Value::Number(Number::from(value));
  let result = bool::parse_value(json_value, &None);
  assert_eq!(value != 0, result.unwrap());

  let value = 123.456;
  let json_value = Value::Number(Number::from_f64(value).unwrap());
  let result = bool::parse_value(json_value, &None);
  assert_eq!(value != 0.0, result.unwrap());

  let json_value = Value::Null;
  let result = bool::parse_value(json_value, &None);
  assert_eq!(
    MessageError::ParameterValueError("Cannot convert value type 'Null' to type bool".to_string()),
    result.unwrap_err()
  );

  let string = "bool_env_var".to_string();
  let json_value = Value::String(string.clone());
  let store_value = true;

  std::env::set_var(&string, &store_value.to_string());
  let result = bool::parse_value(json_value.clone(), &Some("env".to_string()));
  assert_eq!(store_value, result.unwrap());
  std::env::remove_var(&string);

  let result = bool::parse_value(json_value, &Some("env".to_string()));
  assert_eq!(
    MessageError::ParameterValueError("\"environment variable not found\"".to_string()),
    result.unwrap_err()
  );
}
