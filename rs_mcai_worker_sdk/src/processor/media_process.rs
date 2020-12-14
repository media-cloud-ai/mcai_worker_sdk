use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

use crate::{
  job::{Job, JobResult},
  message::media::{
    finish_process, initialize_process,
    output::Output,
    source::{DecodeResult, Source},
  },
  processor::Process,
  publish_job_progression, McaiChannel, MessageError, MessageEvent, Result,
};

#[derive(Default)]
pub struct MediaProcess {
  source: Option<Source>,
  output: Option<Output>,
}

impl<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send> Process<P, ME>
  for MediaProcess
{
  fn init(&mut self, message_event: Arc<Mutex<ME>>, job: &Job) -> Result<()> {
    info!("Initialize job: {:?}", job);

    initialize_process(message_event, &job).map(|(source, output)| {
      self.source = Some(source);
      self.output = Some(output);
    })
  }

  fn start(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    job: &Job,
    feedback_sender: McaiChannel,
  ) -> Result<JobResult> {
    info!("Start processing job: {:?}", job);

    let job_result = JobResult::from(job);
    if let MediaProcess {
      source: Some(source),
      output: Some(output),
    } = self
    {
      info!(
        "{} - Start to process media (start: {} ms, duration: {})",
        job_result.get_str_job_id(),
        source.get_start_offset(),
        source
          .get_segment_duration()
          .map(|duration| format!("{} ms", duration))
          .unwrap_or_else(|| "unknown".to_string())
      );

      let process_duration_ms = source.get_segment_duration();

      let mut processed_frames = 0;
      let mut previous_progress = 0;

      let first_stream_fps = source.get_stream_fps(source.get_first_stream_index()) as f32;

      loop {
        match source.next_frame()? {
          DecodeResult::Frame {
            stream_index,
            frame,
          } => {
            if stream_index == source.get_first_stream_index() {
              processed_frames += 1;

              let processed_ms = processed_frames as f32 * 1000.0 / first_stream_fps;

              if let Some(duration) = process_duration_ms {
                let progress = std::cmp::min((processed_ms / duration as f32 * 100.0) as u8, 100);
                if progress > previous_progress {
                  publish_job_progression(Some(feedback_sender.clone()), job.job_id, progress)?;
                  previous_progress = progress;
                }
              }
            }
            info!(
              "{} - Process frame {}",
              job_result.get_str_job_id(),
              processed_frames
            );

            crate::message::media::process_frame(
              message_event.clone(),
              output,
              job_result.clone(),
              stream_index,
              frame,
            )?;
          }
          DecodeResult::WaitMore => {}
          DecodeResult::Nothing => {}
          DecodeResult::EndOfStream => {
            return finish_process(message_event, output, job_result);
          }
        }
      }
    }

    Err(MessageError::RuntimeError(
      "Process cannot be started, it must be initialized before!".to_string(),
    ))
  }

  fn stop(&mut self, message_event: Arc<Mutex<ME>>, job: &Job) -> Result<JobResult> {
    info!("Stop job: {:?}", job);

    let job_result = JobResult::from(job);

    if let Some(output) = &mut self.output {
      return finish_process(message_event, output, job_result);
    }

    Err(MessageError::RuntimeError(
      "Process must be initialized to be stopped".to_string(),
    ))
  }
}
