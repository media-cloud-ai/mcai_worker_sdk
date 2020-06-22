use crate::{
  error::MessageError::RuntimeError,
  job::{Job, JobResult},
  message::publish_job_progression,
  parameter::container::ParametersContainer,
  McaiChannel, MessageError, MessageEvent,
};
use stainless_ffmpeg::{
  format_context::FormatContext,
  video_decoder::VideoDecoder
};
use std::collections::HashMap;

pub fn process<ME: MessageEvent>(
  message_event: &'static ME,
  channel: Option<McaiChannel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let str_job_id = job.job_id.to_string();

  let filename: String = job.get_parameter("source_path").unwrap();

  let mut format_context = FormatContext::new(&filename).unwrap();
  format_context.open_input().unwrap();

  let selected_streams = message_event.init_process(&format_context)?;

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

  loop {
    match format_context.next_packet() {
      Err(message) => {
        if message == "End of data stream" || message == "Unable to read next packet" {
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
              message_event.process_frame(&str_job_id, stream_index, frame)?;
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
