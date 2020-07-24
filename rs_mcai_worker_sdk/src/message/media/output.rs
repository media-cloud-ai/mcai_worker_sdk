use crate::message::media::srt::SrtStream;
use crate::{MessageError, ProcessResult, Result};
use bytes::Bytes;

pub struct Output {
  srt_stream: Option<SrtStream>,
  results: Vec<ProcessResult>,
  url: String,
}

impl Output {
  pub fn new(output: &str) -> Result<Self> {
    if SrtStream::is_srt_stream(output) {
      let srt_stream = Some(SrtStream::open_connection(output)?);

      Ok(Output {
        srt_stream,
        results: vec![],
        url: output.to_string(),
      })
    } else {
      Ok(Output {
        srt_stream: None,
        results: vec![],
        url: output.to_string(),
      })
    }
  }

  pub fn push(&mut self, content: ProcessResult) {
    if let Some(srt_stream) = &mut self.srt_stream {
      let data = Bytes::from(content.json_content.unwrap_or_else(|| "{}".to_string()));
      srt_stream.send(data);
    } else {
      self.results.push(content);
    }
  }

  pub fn to_destination_path(&self) -> Result<()> {
    let json_results: Vec<serde_json::Value> = self
      .results
      .iter()
      .filter(|result| result.json_content.is_some())
      .map(|result| serde_json::from_str(&result.json_content.as_ref().unwrap()).unwrap())
      .collect();

    let content = if !json_results.is_empty() {
      serde_json::to_string(&json!({
        "frames": json_results,
      }))
      .unwrap()
    } else {
      self
        .results
        .iter()
        .filter(|result| result.xml_content.is_some())
        .map(|result| result.xml_content.as_ref().unwrap().clone())
        .collect::<Vec<String>>()
        .join("")
    };

    std::fs::write(self.url.clone(), content).map_err(|error| {
      MessageError::RuntimeError(format!(
        "Could not write to '{}' destination: {:?}",
        self.url.clone(),
        error
      ))
    })?;

    Ok(())
  }
}

#[test]
pub fn test_output() {
  let url = "/path/to/somewhere";
  let mut output = Output::new(url).unwrap();

  assert!(output.srt_stream.is_none());
  assert_eq!(0, output.results.len());
  assert_eq!(url, output.url);

  let process_result = ProcessResult::new_json("{\"status\": \"OK\"}");
  output.push(process_result);
  assert_eq!(1, output.results.len());

  let result = output.to_destination_path();
  assert!(result.is_err());

  let expected_error = MessageError::RuntimeError(format!("Could not write to '/path/to/somewhere' destination: Os {{ code: 2, kind: NotFound, message: \"No such file or directory\" }}"));
  assert_eq!(expected_error, result.unwrap_err());
}
