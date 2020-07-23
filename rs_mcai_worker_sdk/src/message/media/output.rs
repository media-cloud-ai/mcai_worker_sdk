use crate::message::media::srt::SrtStream;
use crate::{ProcessResult, Result};
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

    std::fs::write(self.url.clone(), content).unwrap();

    Ok(())
  }
}
