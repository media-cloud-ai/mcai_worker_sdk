use bytes::Bytes;
use crate::{
  error::MessageError::RuntimeError,
  job::{Job, JobResult},
  message::publish_job_progression,
  parameter::container::ParametersContainer,
  McaiChannel, MessageError, MessageEvent,
  ProcessResult,
};
use futures_util::sink::SinkExt;
use srt::tokio::SrtSocket;
use srt::SrtSocketBuilder;
use stainless_ffmpeg::{
  format_context::FormatContext,
  video_decoder::VideoDecoder
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
use tokio::runtime::Runtime;

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
      let srt_socket =
        runtime.block_on(async {
          if output.starts_with("srt://:") {
            let port = output.replace("srt://:", "").parse::<u16>().unwrap();
            SrtSocketBuilder::new_listen()
              .local_port(port)
              .connect()
              .await
              .unwrap()
          } else {
            let url = output.replace("srt://", "");

            SrtSocketBuilder::new_connect(url)
              .connect()
              .await
              .unwrap()
          }
        });

      info!("SRT connected");

      Output {
        srt_stream: Some(Rc::new(RefCell::new(srt_socket))),
        results: vec![],
        runtime,
        url: output.to_string()
      }
    } else {
      Output {
        srt_stream: None,
        results: vec![],
        runtime,
        url: output.to_string()
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

        if let Err(reason) =
          srt_stream
            .clone()
            .borrow_mut()
            .send((Instant::now(), Bytes::from(content.content.unwrap_or_else(|| "{}".to_string()))))
            .await {
              error!("unable to send message, reason: {}", reason);
            }
      });
    }
  }

  fn to_destination_path(&self) -> Result<(), MessageError> {
    let results: Vec<serde_json::Value> =
      self.results
        .iter()
        .filter(|result| result.content.is_some())
        .map(|result|
          serde_json::from_str(&result.content.as_ref().unwrap()).unwrap()
        )
        .collect();

    let content = json!({
      "frames": results,
    });

    std::fs::write(self.url.clone(), serde_json::to_string(&content).unwrap()).unwrap();

    Ok(())
  }
}

pub fn process<ME: MessageEvent>(
  message_event: Rc<RefCell<ME>>,
  channel: Option<McaiChannel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let str_job_id = job.job_id.to_string();

  let filename: String = job.get_parameter("source_path").unwrap();
  let output_url: String = job.get_parameter("destination_path").unwrap();

  let mut format_context = FormatContext::new(&filename).unwrap();
  format_context.open_input().unwrap();

  let selected_streams = message_event.borrow_mut().init_process(job, &format_context)?;

  info!(target: &str_job_id, "Selected stream IDs: {:?}", selected_streams);

  let mut decoders : HashMap<usize, VideoDecoder> = HashMap::new();

  for selected_stream in &selected_streams {
    // VideoDecoder can decode any codec, not only video
    let decoder = VideoDecoder::new(format!("decoder_{}", selected_stream), &format_context, *selected_stream as isize).unwrap();
    decoders.insert(*selected_stream, decoder);
  }

  info!(target: &str_job_id, "Start to process media");

  let total_duration = format_context.get_duration().map(|duration| duration * 25.0);
  let mut count = 0;
  let mut previous_progress = 0;

  let mut output = Output::new(&output_url);

  loop {
    match format_context.next_packet() {
      Err(message) => {
        if message == "End of data stream" || message == "Unable to read next packet" {
          output.to_destination_path()?;
          return Ok(job_result);
        }

        return Err(RuntimeError(message));
      }
      Ok(packet) => {
        let stream_index = packet.get_stream_index() as usize;
        if stream_index == 0 && total_duration.is_some() {
          count += 1;

          if let Some(duration) = total_duration {
            let progress = (count as f64 / duration * 100.0) as u8;
            if progress > previous_progress {
              publish_job_progression(channel.clone(), &job, progress)?;
              previous_progress = progress;
            }
          }
        }

        if let Some(decoder) = decoders.get(&stream_index) {
          match decoder.decode(&packet) {
            Ok(frame) => {
              trace!(target: &job_result.get_str_job_id(), "Process frame {}", count);
              let result = message_event.borrow_mut().process_frame(&str_job_id, stream_index, frame)?;

              output.push(result);
            }
            Err(message) => {
              if message == "Resource temporarily unavailable" {
                continue;
              }
              return Err(RuntimeError(message));
            }
          }
        }
      }
    }
  }
}
