use bytes::Bytes;
use futures::{stream, SinkExt, StreamExt};
use mcai_worker_sdk::prelude::*;
use srt_tokio::SrtSocketBuilder;
use std::{
  time::{Duration, Instant},
};
use tokio::time::sleep;

pub struct SrtStreamGenerator;

impl SrtStreamGenerator {
  pub async fn new_json(port: u16) -> Result<()> {
    let mut srt_socket = SrtSocketBuilder::new_listen()
      .local_port(port)
      .connect()
      .await
      .unwrap();

    let mut stream = stream::unfold(0, |count| async move {
      sleep(Duration::from_millis(10)).await;

      // let data = vec![0; 8000];
      let data = r#"{"key": "value"}"#;
      let payload = Bytes::from(data);

      return Some((Ok((Instant::now(), payload)), count + 1));
    })
    .boxed();

    srt_socket.send_all(&mut stream).await.unwrap();
    Ok(())
  }
}
