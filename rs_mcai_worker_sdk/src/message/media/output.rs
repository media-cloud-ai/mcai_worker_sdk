use std::{cell::RefCell, rc::Rc};

struct Output {
  srt_stream: Option<Rc<RefCell<SrtSocket>>>,
  results: Vec<ProcessResult>,
  runtime: Runtime,
  url: String,
}

impl Output {
  fn new(output: &str) -> Self {
    let mut runtime = Runtime::new().unwrap();

    if output.starts_with("srt://") {
      let srt_socket = runtime.block_on(async {
        if output.starts_with("srt://:") {
          let port = output.replace("srt://:", "").parse::<u16>().unwrap();
          SrtSocketBuilder::new_listen()
            .local_port(port)
            .connect()
            .await
            .unwrap()
        } else {
          let url = output.replace("srt://", "");

          SrtSocketBuilder::new_connect(url).connect().await.unwrap()
        }
      });

      info!("SRT connected");

      Output {
        srt_stream: Some(Rc::new(RefCell::new(srt_socket))),
        results: vec![],
        runtime,
        url: output.to_string(),
      }
    } else {
      Output {
        srt_stream: None,
        results: vec![],
        runtime,
        url: output.to_string(),
      }
    }
  }

  fn push(&mut self, content: ProcessResult) {
    if self.srt_stream.is_none() {
      self.results.push(content);
      return;
    }

    if let Some(srt_stream) = &self.srt_stream {
      self.runtime.block_on(async {
        if let Err(reason) = srt_stream
          .clone()
          .borrow_mut()
          .send((
            Instant::now(),
            Bytes::from(content.content.unwrap_or_else(|| "{}".to_string())),
          ))
          .await
        {
          error!("unable to send message, reason: {}", reason);
        }
      });
    }
  }

  fn to_destination_path(&self) -> Result<(), MessageError> {
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
