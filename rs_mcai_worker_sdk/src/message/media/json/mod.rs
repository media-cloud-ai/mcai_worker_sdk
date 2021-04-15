extern crate serde_json;

pub mod decoder;

pub use decoder::JsonDecoder;
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub struct Json {
  pub value: Option<Value>,
}

#[test]
#[cfg(feature = "media")]
fn test_json_deserialization() {
  let mut json_decoder = JsonDecoder::new();
  let input_string = "{\"test\":\"this is a test\"}";
  let value: Value = serde_json::from_str(input_string).unwrap();
  assert_eq!(
    Some(value.clone()),
    json_decoder
      .decode_content(input_string)
      .unwrap()
      .unwrap()
      .value
  );
  let input_string = "{\"test\":\"this is a";
  assert_eq!(None, json_decoder.decode_content(input_string).unwrap());
  let input_string = " test\"}{\"test\":\"";
  assert_eq!(
    Some(value.clone()),
    json_decoder
      .decode_content(input_string)
      .unwrap()
      .unwrap()
      .value
  );
  let input_string = "this is a test\"}";
  assert_eq!(
    Some(value.clone()),
    json_decoder
      .decode_content(input_string)
      .unwrap()
      .unwrap()
      .value
  );
}

#[test]
#[cfg(feature = "media")]
fn test_stringified_json() {
  let mut json_decoder = JsonDecoder::new();
  let input_string = "\"{\\\"test\\\":\\\"this is a test\\\"}\"";
  assert_eq!(
    json_decoder.decode_content(input_string),
    Err(format!("Incorrect JSON content: {}", input_string))
  );
}
