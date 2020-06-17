use crate::{
  error::MessageError::RuntimeError,
  job::{Job, JobResult},
  message::publish_job_progression,
  parameter::container::ParametersContainer,
  McaiChannel, MessageError, MessageEvent,
};

use stainless_ffmpeg::{format_context::FormatContext, video_decoder::VideoDecoder};

pub fn process<ME: MessageEvent>(
  message_event: &'static ME,
  channel: Option<McaiChannel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let filename: String = job.get_parameter("source_path").unwrap();

  let mut context = FormatContext::new(&filename).unwrap();
  context.open_input().unwrap();

  let video_decoder = VideoDecoder::new("h264".to_string(), &context, 0).unwrap();

  info!("Start to process media");

  let total_duration = context.get_duration().map(|duration| duration * 25.0);
  let mut count = 0;
  let mut previous_progress = 0;

  loop {
    match context.next_packet() {
      Err(message) => {
        if message == "End of data stream" || message == "Unable to read next packet" {
          return Ok(job_result);
        }

        return Err(RuntimeError(message));
      }
      Ok(packet) => {
        if packet.get_stream_index() == 0 && total_duration.is_some() {
          count += 1;

          if let Some(duration) = total_duration {
            let progress = (count as f64 / duration * 100.0) as u8;
            if progress > previous_progress {
              publish_job_progression(channel.clone(), &job, progress)?;
              previous_progress = progress;
            }
          }
        }

        // let frame = video_decoder.decode(&packet).unwrap();

        message_event.process_frame().unwrap();
      }
    }
  }
}
