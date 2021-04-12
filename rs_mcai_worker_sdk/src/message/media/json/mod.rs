extern crate serde_json;

pub mod decoder;

pub use decoder::JsonDecoder;
use serde_json::Value;

pub struct Json {
  pub value: Option<Value>,
}
