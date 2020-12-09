use serde::Serialize;
use yaserde::YaSerialize;

#[derive(Debug)]
pub struct ProcessResult {
  pub end_of_process: bool,
  pub json_content: Option<String>,
  pub xml_content: Option<String>,
}

impl ProcessResult {
  pub fn empty() -> Self {
    ProcessResult {
      end_of_process: false,
      json_content: None,
      xml_content: None,
    }
  }

  pub fn end_of_process() -> Self {
    ProcessResult {
      end_of_process: true,
      json_content: None,
      xml_content: None,
    }
  }

  pub fn new_json<S: Serialize>(content: S) -> Self {
    let content = serde_json::to_string(&content).unwrap();

    ProcessResult {
      end_of_process: false,
      json_content: Some(content),
      xml_content: None,
    }
  }

  pub fn new_xml<Y: YaSerialize>(content: Y) -> Self {
    let content = yaserde::ser::to_string(&content).unwrap();

    ProcessResult {
      end_of_process: false,
      json_content: None,
      xml_content: Some(content),
    }
  }
}
