use std::rc::Rc;

use failure::_core::cell::RefCell;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use crate::job::{Job, JobResult};
use crate::message::media::initialize_process;
use crate::message::media::output::Output;
use crate::message::media::source::{DecodeResult, Source};
use crate::processor::Process;
use crate::{MessageError, MessageEvent, Result};

#[derive(Default)]
pub struct MediaProcess {
  source: Option<Source>,
  output: Option<Output>,
}

impl Process for MediaProcess {
  fn init<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Rc<RefCell<ME>>,
    job: &Job,
  ) -> Result<()> {
    info!("Initialize job: {:?}", job);

    initialize_process(message_event, &job).map(|(source, output)| {
      self.source = Some(source);
      self.output = Some(output);
    })
  }

  fn start<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Rc<RefCell<ME>>,
    job: &Job,
  ) -> Result<JobResult> {
    info!("Start processing job: {:?}", job);

    let job_result = JobResult::from(job);
    if let MediaProcess {
      source: Some(source),
      output: Some(output),
    } = self
    {
      println!(
        "{} - Start to process media (start: {} ms, duration: {})",
        job_result.get_str_job_id(),
        source.get_start_offset(),
        source
          .get_segment_duration()
          .map(|duration| format!("{} ms", duration))
          .unwrap_or_else(|| "unknown".to_string())
      );

      let total_duration = source.get_duration();
      let mut count = 0;
      let mut previous_progress = 0;

      loop {
        match source.next_frame()? {
          DecodeResult::Frame {
            stream_index,
            frame,
          } => {
            println!(">> Frame...");
            if stream_index == source.get_first_stream_index() {
              count += 1;

              if let Some(duration) = total_duration {
                let progress = std::cmp::min((count / duration * 100) as u8, 100);
                if progress > previous_progress {
                  println!("Progress: {:?}", progress);
                  previous_progress = progress;
                }
              }
            }
            println!("{} - Process frame {}", job_result.get_str_job_id(), count);

            crate::message::media::process_frame(
              message_event.clone(),
              output,
              job_result.clone(),
              stream_index,
              frame,
            )?;
          }
          DecodeResult::WaitMore => {
            println!(">> Wait more...");
          }
          DecodeResult::Nothing => {
            println!(">> Nothing...");
          }
          DecodeResult::EndOfStream => {
            println!(">> EndOfStream...");
            return crate::message::media::finish_process(message_event, output, job_result);
          }
        }
      }
    }

    Err(MessageError::RuntimeError(
      "Process cannot be started, it must be initialized before!".to_string(),
    ))
  }

  fn stop<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Rc<RefCell<ME>>,
    job: &Job,
  ) -> Result<JobResult> {
    info!("Stop job: {:?}", job);

    let job_result = JobResult::from(job);

    if let Some(output) = &mut self.output {
      return crate::message::media::finish_process(message_event, output, job_result);
    }

    Err(MessageError::RuntimeError(
      "Process must be initialized to be stopped".to_string(),
    ))
  }
}
