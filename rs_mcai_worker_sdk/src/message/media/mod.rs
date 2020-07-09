use crate::{
  job::{Job, JobResult},
  message::publish_job_progression,
  parameter::container::ParametersContainer,
  McaiChannel, MessageError, MessageEvent,
};
use std::cell::RefCell;
use std::rc::Rc;

mod output;
mod source;
mod srt;
use source::DecodeResult;

pub fn process<ME: MessageEvent>(
  message_event: Rc<RefCell<ME>>,
  channel: Option<McaiChannel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let str_job_id = job.job_id.to_string();

  let source_url: String = job.get_parameter("source_path").unwrap();
  let output_url: String = job.get_parameter("destination_path").unwrap();

  let mut source = source::Source::new(message_event.clone(), job, &source_url)?;

  info!(target: &str_job_id, "Start to process media");

  let total_duration = source.get_duration();
  let mut count = 0;
  let mut previous_progress = 0;

  let mut output = output::Output::new(&output_url)?;

  loop {
    match source.next_frame()? {
      DecodeResult::Frame {
        stream_index,
        frame,
      } => {
        count += 1;

        if stream_index == 0 {
          count += 1;

          if let Some(duration) = total_duration {
            let progress = std::cmp::min((count as f64 / duration * 100.0) as u8, 100);
            if progress > previous_progress {
              publish_job_progression(channel.clone(), &job, progress)?;
              previous_progress = progress;
            }
          }
        }

        trace!(target: &job_result.get_str_job_id(), "Process frame {}", count);
        let result = message_event
          .borrow_mut()
          .process_frame(&str_job_id, stream_index, frame)?;

        output.push(result);
      }
      DecodeResult::Nothing => {}
      DecodeResult::EndOfStream => {
        output.to_destination_path()?;
        return Ok(job_result);
      }
    }
  }
}
