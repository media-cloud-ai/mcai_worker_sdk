use crate::message::media::srt::SrtStream;
use crate::{MessageError, ProcessResult, Result};
use bytes::Bytes;
use std::{
  sync::{
    mpsc::{channel, Sender},
    Arc, Mutex,
  },
  thread::JoinHandle,
};

pub struct Output {
  results: Arc<Mutex<Vec<ProcessResult>>>,
  url: String,
  thread: Option<JoinHandle<()>>,
  sender: Arc<Mutex<Sender<ProcessResult>>>,
}

impl Output {
  pub fn new(output: &str) -> Result<Self> {
    let (sender, receiver) = channel::<ProcessResult>();
    let output = output.to_string();
    let url = output.clone();

    let results = Arc::new(Mutex::new(vec![]));
    let cloned_results = results.clone();

    let thread = Some(std::thread::spawn(move || {
      let mut srt_stream = if SrtStream::is_srt_stream(&output) {
        Some(SrtStream::open_connection(&output).unwrap())
      } else {
        None
      };

      while let Ok(message) = receiver.recv() {
        match message {
          ProcessResult {
            end_of_process: true,
            ..
          } => break,
          ProcessResult {
            json_content: Some(content),
            ..
          } => {
            info!("[Output] Json message {}", content);
            if let Some(srt_stream) = &mut srt_stream {
              let data = Bytes::from(content);
              srt_stream.send(data);
            } else {
              let message = ProcessResult {
                json_content: Some(content),
                xml_content: None,
                end_of_process: false,
              };

              cloned_results.lock().unwrap().push(message);
            }
          }
          ProcessResult {
            xml_content: Some(content),
            ..
          } => {
            info!("[Output] XML message {}", content);
            if let Some(srt_stream) = &mut srt_stream {
              let data = Bytes::from(content);
              srt_stream.send(data);
            } else {
              let message = ProcessResult {
                json_content: None,
                xml_content: Some(content),
                end_of_process: false,
              };

              cloned_results.lock().unwrap().push(message);
            }
          }
          ProcessResult {
            end_of_process: false,
            json_content: None,
            xml_content: None,
          } => {}
        }
      }

      if let Some(mut srt_stream) = srt_stream {
        srt_stream.close();
      }

      info!("End of output thread");
    }));
    let sender = Arc::new(Mutex::new(sender));

    Ok(Output {
      results,
      url,
      thread,
      sender,
    })
  }

  pub fn push(&mut self, content: ProcessResult) {
    self.sender.lock().unwrap().send(content).unwrap();
  }

  pub fn get_sender(&self) -> Arc<Mutex<Sender<ProcessResult>>> {
    self.sender.clone()
  }

  pub fn complete(&mut self) -> Result<()> {
    self.thread.take().map(JoinHandle::join);

    if SrtStream::is_srt_stream(&self.url) {
      return Ok(());
    }

    let json_results: Vec<serde_json::Value> = self
      .results
      .lock()
      .unwrap()
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
        .lock()
        .unwrap()
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

  assert_eq!(0, output.results.lock().unwrap().len());
  assert_eq!(url, output.url);

  let ok_content = r#"{"status": "OK"}"#;
  let process_result = ProcessResult::new_json(ok_content);
  output.push(process_result);

  let process_result = ProcessResult::end_of_process();
  output.push(process_result);

  // wait a bit for the result to be received...
  std::thread::sleep(std::time::Duration::from_millis(10));

  {
    // check results
    let results_ref = output.results.lock().unwrap();
    assert_eq!(1, results_ref.len());

    let process_result = results_ref.get(0).unwrap();
    let expected_result = ProcessResult::new_json(ok_content);
    assert_eq!(
      process_result.end_of_process,
      expected_result.end_of_process
    );
    assert_eq!(process_result.json_content, expected_result.json_content);
    assert_eq!(process_result.xml_content, expected_result.xml_content);
  }

  let result = output.complete();
  assert!(result.is_err());

  let expected_error = MessageError::RuntimeError(format!("Could not write to '/path/to/somewhere' destination: Os {{ code: 2, kind: NotFound, message: \"No such file or directory\" }}"));
  assert_eq!(expected_error, result.unwrap_err());
}
