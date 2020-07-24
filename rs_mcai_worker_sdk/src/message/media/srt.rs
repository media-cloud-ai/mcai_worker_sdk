use crate::Result;
use bytes::Bytes;
use futures_util::sink::SinkExt;
use srt::tokio::SrtSocket;
use srt::SrtSocketBuilder;
use std::{cell::RefCell, rc::Rc, time::Instant};
use tokio::{runtime::Runtime, stream::StreamExt};

pub struct SrtStream {
  socket: Rc<RefCell<SrtSocket>>,
  runtime: Runtime,
}

impl SrtStream {
  pub fn is_srt_stream(url: &str) -> bool {
    url.starts_with("srt://")
  }

  pub fn open_connection(url: &str) -> Result<SrtStream> {
    let mut runtime = Runtime::new().unwrap();

    let socket = runtime.block_on(async {
      if url.starts_with("srt://:") {
        let port = url.replace("srt://:", "").parse::<u16>().unwrap();
        SrtSocketBuilder::new_listen()
          .local_port(port)
          .connect()
          .await
          .unwrap()
      } else {
        let url = url.replace("srt://", "");

        SrtSocketBuilder::new_connect(url).connect().await.unwrap()
      }
    });

    let socket = Rc::new(RefCell::new(socket));

    info!("SRT connected");
    Ok(SrtStream { socket, runtime })
  }

  pub fn send(&mut self, data: Bytes) {
    let socket = self.socket.clone();
    self.runtime.block_on(async {
      if let Err(reason) = socket.borrow_mut().send((Instant::now(), data)).await {
        error!("unable to send message, reason: {}", reason);
      }
    });
  }

  pub fn receive(&mut self) -> Option<(Instant, Bytes)> {
    let socket = self.socket.clone();
    self
      .runtime
      .block_on(async { socket.borrow_mut().try_next().await.unwrap() })
  }
}

#[test]
pub fn test_is_srt_stream() {
  let file_name = "file.txt";
  assert!(!SrtStream::is_srt_stream(file_name));
  let file_path = "/path/to/file";
  assert!(!SrtStream::is_srt_stream(file_path));
  let file_url = "file://path/to/file";
  assert!(!SrtStream::is_srt_stream(file_url));
  let http_url = "http://path/to/resource";
  assert!(!SrtStream::is_srt_stream(http_url));

  let srt_url = "srt://path/to/resource";
  assert!(SrtStream::is_srt_stream(srt_url));
}
