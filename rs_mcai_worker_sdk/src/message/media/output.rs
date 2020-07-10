use crate::message::media::srt::SrtStream;
use crate::{MessageError, ProcessResult};
use bytes::Bytes;

pub struct Output {
  srt_stream: Option<SrtStream>,
  results: Vec<ProcessResult>,
  url: String,
}

impl Output {
  pub fn new(output: &str) -> Result<Self, MessageError> {
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
      let data = Bytes::from(content.content.unwrap_or_else(|| "{}".to_string()));
      srt_stream.send(data);
    } else {
      self.results.push(content);
    }
  }

  pub fn to_destination_path(&self) -> Result<(), MessageError> {
    let results: Vec<serde_json::Value> = self
      .results
      .iter()
      .filter(|result| result.content.is_some())
      .map(|result| serde_json::from_str(&result.content.as_ref().unwrap()).unwrap())
      .collect();

    let content = json!({
      "frames": results,
    });

    std::fs::write(self.url.clone(), serde_json::to_string(&content).unwrap()).unwrap();

    Ok(())
  }
}
