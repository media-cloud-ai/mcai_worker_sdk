#[macro_use]
extern crate serde_derive;

use mcai_worker_sdk::{
  job::{JobResult, JobStatus},
  publish_job_progression, McaiChannel, MessageError, MessageEvent, Result,
};
use schemars::JsonSchema;
use semver::Version;
use std::{thread::sleep, time::Duration};

#[cfg(feature = "media")]
use {
  mcai_worker_sdk::{
    info, AudioFilter, AudioFormat, FormatContext, ProcessFrame, ProcessResult, Scaling,
    StreamDescriptor, VideoFilter,
  },
  ops::Deref,
  stainless_ffmpeg_sys::AVMediaType,
  sync::{mpsc::Sender, Arc, Mutex},
};

#[derive(Debug, Deserialize, JsonSchema)]
struct WorkerParameters {
  action: Option<String>,
  source_path: Option<String>,
  destination_path: Option<String>,
  /// Option sleep time in milliseconds
  ///
  /// For not media, it will sleep until a stop is received
  ///
  /// For media it will be between each frame
  sleep: Option<u64>,
}

#[derive(Debug, Default)]
struct WorkerContext {
  #[cfg(feature = "media")]
  result: Option<Arc<Mutex<Sender<ProcessResult>>>>,
  #[cfg(feature = "media")]
  sleep: Option<u64>,
}

impl MessageEvent<WorkerParameters> for WorkerContext {
  fn get_name(&self) -> String {
    "Example".to_string()
  }

  fn get_short_description(&self) -> String {
    "An example worker".to_string()
  }

  fn get_description(&self) -> String {
    r#"This worker is just an example to demonstrate the API of rs_amqp_worker.
Do no use in production, just for developments."#
      .to_string()
  }

  fn get_version(&self) -> Version {
    Version::new(1, 2, 3)
  }

  fn init(&mut self) -> Result<()> {
    Ok(())
  }

  #[cfg(feature = "media")]
  fn init_process(
    &mut self,
    parameters: WorkerParameters,
    format_context: Arc<Mutex<FormatContext>>,
    result: Arc<Mutex<Sender<ProcessResult>>>,
  ) -> Result<Vec<StreamDescriptor>> {
    self.result = Some(result);
    self.sleep = parameters.sleep;

    let mut stream_descriptors = vec![];

    let format_context = format_context.lock().unwrap();
    for stream_index in 0..format_context.get_nb_streams() {
      let stream_type = format_context.get_stream_type(stream_index as isize);
      info!(
        "Handle stream #{} with type: {:?}",
        stream_index, stream_type
      );

      match stream_type {
        AVMediaType::AVMEDIA_TYPE_VIDEO => {
          let filters = vec![VideoFilter::Resize(Scaling {
            width: Some(200),
            height: Some(70),
          })];
          stream_descriptors.push(StreamDescriptor::new_video(stream_index as usize, filters))
        }
        AVMediaType::AVMEDIA_TYPE_AUDIO => {
          let channel_layouts = vec!["mono".to_string()];
          let sample_formats = vec!["s16".to_string()];
          let sample_rates = vec![16000];

          let filters = vec![AudioFilter::Format(AudioFormat {
            sample_rates,
            channel_layouts,
            sample_formats,
          })];
          stream_descriptors.push(StreamDescriptor::new_audio(stream_index as usize, filters))
        }
        AVMediaType::AVMEDIA_TYPE_SUBTITLE => {
          stream_descriptors.push(StreamDescriptor::new_data(stream_index as usize))
        }
        AVMediaType::AVMEDIA_TYPE_DATA => {
          stream_descriptors.push(StreamDescriptor::new_data(stream_index as usize))
        }
        _ => info!("Skip stream #{}", stream_index),
      };
    }
    Ok(stream_descriptors)
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &mut self,
    job_result: JobResult,
    stream_index: usize,
    frame: ProcessFrame,
  ) -> Result<ProcessResult> {
    match frame {
      ProcessFrame::AudioVideo(frame) => {
        unsafe {
          let width = (*frame.frame).width;
          let height = (*frame.frame).height;
          let sample_rate = (*frame.frame).sample_rate;
          let channels = (*frame.frame).channels;
          let nb_samples = (*frame.frame).nb_samples;

          if width != 0 && height != 0 {
            info!(
              target: &job_result.get_str_job_id(),
              "Stream {} - PTS: {}, image size: {}x{}",
              stream_index,
              frame.get_pts(),
              width,
              height
            );
          } else {
            info!(
              target: &job_result.get_str_job_id(),
              "Stream {} - PTS: {}, sample_rate: {}Hz, channels: {}, nb_samples: {}",
              stream_index,
              frame.get_pts(),
              sample_rate,
              channels,
              nb_samples,
            );
          }
        }

        if let Some(duration) = self.sleep {
          sleep(Duration::from_millis(duration));
        }

        Ok(ProcessResult::new_json(""))
      }
      ProcessFrame::EbuTtmlLive(ebu_ttml_live) => {
        Ok(ProcessResult::new_xml(ebu_ttml_live.deref().clone()))
      }
      _ => Err(MessageError::NotImplemented()),
    }
  }

  #[cfg(feature = "media")]
  fn ending_process(&mut self) -> Result<()> {
    if let Some(result) = &self.result {
      result
        .lock()
        .unwrap()
        .send(ProcessResult::end_of_process())
        .unwrap();
    }
    Ok(())
  }

  /// Not called when the "media" feature is enabled
  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: WorkerParameters,
    job_result: JobResult,
  ) -> Result<JobResult> {
    publish_job_progression(channel.clone(), job_result.get_job_id(), 50)?;

    if let Some(duration) = parameters.sleep {
      loop {
        if let Some(channel) = &channel {
          if channel.lock().unwrap().is_stopped() {
            return Ok(job_result.with_status(JobStatus::Stopped));
          }
        }
        log::debug!("sleep more ...");
        sleep(Duration::from_millis(duration));
      }
    }

    match parameters.action {
      Some(action_label) => match action_label.as_str() {
        "completed" => {
          publish_job_progression(channel, job_result.get_job_id(), 100)?;
          Ok(job_result.with_status(JobStatus::Completed))
        }
        action_label => {
          let result = job_result.with_message(&format!("Unknown action named {}", action_label));
          Err(MessageError::ProcessingError(result))
        }
      },
      None => {
        let result = job_result.with_message(&format!("Unspecified action parameter"));
        Err(MessageError::ProcessingError(result))
      }
    }
  }
}

fn main() {
  let worker_context = WorkerContext::default();
  mcai_worker_sdk::start_worker(worker_context);
}
