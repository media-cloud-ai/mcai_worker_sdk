use crate::{
  job::{Job, JobProgression, JobResult, JobStatus},
  message::media::{
    finish_process, initialize_process,
    output::Output,
    source::{DecodeResult, Source},
  },
  message_exchange::message::{Feedback, OrderMessage, ResponseMessage},
  processor::ProcessStatus,
  publish_job_progression,
  worker::{SystemInformation, WorkerActivity, WorkerConfiguration, WorkerStatus},
  McaiChannel, MessageError, MessageEvent, Result,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{mpsc::Receiver, Arc, Mutex};

pub struct ThreadedMediaProcess {
  source: Source,
  output: Output,
  keep_running: bool,
  pub job: Job,
}

impl ThreadedMediaProcess {
  pub fn initialize_process<
    P: DeserializeOwned + JsonSchema,
    ME: 'static + MessageEvent<P> + Send,
  >(
    message_event: Arc<Mutex<ME>>,
    job: Job,
  ) -> Result<Self> {
    log::info!("Initialize job: {:?}", job);

    initialize_process(message_event, &job).map(|(source, output)| ThreadedMediaProcess {
      source,
      output,
      keep_running: false,
      job,
    })
  }

  pub fn get_status_feedback(
    &self,
    status: JobStatus,
    worker_configuration: WorkerConfiguration,
  ) -> ResponseMessage {
    let job_result = JobResult::new(self.job.job_id).with_status(status.clone());

    get_status_feedback(status, Some(job_result), worker_configuration)
  }

  pub fn start_process<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    order_receiver: &Receiver<OrderMessage>,
    response_sender: McaiChannel,
    worker_configuration: WorkerConfiguration,
  ) -> ResponseMessage {
    let job = self.job.clone();
    let job_result = JobResult::from(job.clone());

    response_sender
      .lock()
      .unwrap()
      .send_response(ResponseMessage::WorkerStarted(job_result.clone()))
      .unwrap();

    log::info!("Start processing job: {:?}", job);

    // start publishing progression
    response_sender
      .lock()
      .unwrap()
      .send_response(ResponseMessage::Feedback(Feedback::Progression(
        JobProgression::new(job.job_id, 0),
      )))
      .unwrap();

    log::info!(
      "{} - Start to process media (start: {} ms, duration: {})",
      job_result.get_str_job_id(),
      self.source.get_start_offset(),
      self
        .source
        .get_segment_duration()
        .map(|duration| format!("{} ms", duration))
        .unwrap_or_else(|| "unknown".to_string())
    );

    let process_duration_ms = self.source.get_segment_duration();

    let mut processed_frames = 0;
    let mut previous_progress = 0;

    let first_stream_fps = self
      .source
      .get_stream_fps(self.source.get_first_stream_index()) as f32;

    loop {
      if response_sender.lock().unwrap().is_stopped() {
        log::info!("Stopped !");
        break ResponseMessage::JobStopped(job_result.with_status(JobStatus::Stopped));
      }

      // Process next frame
      let response = self
        .process_frame(
          message_event.clone(),
          job_result.clone(),
          response_sender.clone(),
          first_stream_fps,
          process_duration_ms,
          &mut processed_frames,
          &mut previous_progress,
        )
        .unwrap_or_else(|error| Some(ResponseMessage::Error(error)));

      // If a message is returned, stop looping and forward the message
      if let Some(message) = response {
        break message;
      }

      // Otherwise check whether an order message as been sent to this thread
      if let Ok(message) = order_receiver.try_recv() {
        let resp = match message {
          OrderMessage::Job(_) => ResponseMessage::Error(MessageError::ProcessingError(
            job_result
              .clone()
              .with_status(JobStatus::Running)
              .with_message("Cannot handle a job while a process is running"),
          )),
          OrderMessage::InitProcess(_) => ResponseMessage::Error(MessageError::ProcessingError(
            job_result
              .clone()
              .with_status(JobStatus::Running)
              .with_message("Cannot initialize a running process"),
          )),
          OrderMessage::StartProcess(_) => ResponseMessage::Error(MessageError::ProcessingError(
            job_result
              .clone()
              .with_status(JobStatus::Running)
              .with_message("Cannot start a running process"),
          )),
          OrderMessage::StopProcess(_) => {
            break finish_process(message_event, &mut self.output, JobResult::from(job))
              .map(ResponseMessage::Completed)
              .unwrap_or_else(ResponseMessage::Error);
          }
          OrderMessage::Status => {
            self.get_status_feedback(JobStatus::Running, worker_configuration.clone())
          }
          OrderMessage::StopWorker => {
            self.keep_running = false;
            self.get_status_feedback(JobStatus::Running, worker_configuration.clone())
          }
        };

        response_sender.lock().unwrap().send_response(resp).unwrap();
      }
    }
  }

  pub fn process_frame<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    job_result: JobResult,
    response_sender: McaiChannel,
    first_stream_fps: f32,
    process_duration_ms: Option<u64>,
    processed_frames: &mut usize,
    previous_progress: &mut u8,
  ) -> Result<Option<ResponseMessage>> {
    let response = match self.source.next_frame()? {
      DecodeResult::Frame {
        stream_index,
        frame,
      } => {
        if stream_index == self.source.get_first_stream_index() {
          (*processed_frames) += 1;

          let processed_ms = (*processed_frames) as f32 * 1000.0 / first_stream_fps;

          if let Some(duration) = process_duration_ms {
            let progress = std::cmp::min((processed_ms / duration as f32 * 100.0) as u8, 100);
            if progress > (*previous_progress) {
              publish_job_progression(Some(response_sender), job_result.get_job_id(), progress)?;
              (*previous_progress) = progress;
            }
          }
        }
        log::info!(
          "{} - Process frame {}",
          job_result.get_str_job_id(),
          processed_frames
        );

        let _process_result = crate::message::media::process_frame(
          message_event,
          &mut self.output,
          job_result,
          stream_index,
          frame,
        )?;

        None
      }
      DecodeResult::WaitMore | DecodeResult::Nothing => None,
      DecodeResult::EndOfStream => {
        log::debug!("Media Process: End Of Stream");
        let response = finish_process(message_event, &mut self.output, job_result)
          .map(ResponseMessage::Completed)?;

        Some(response)
      }
    };

    Ok(response)
  }
}

fn get_status_feedback(
  status: JobStatus,
  job_result: Option<JobResult>,
  worker_configuration: WorkerConfiguration,
) -> ResponseMessage {
  let activity = match &status {
    JobStatus::Initialized | JobStatus::Running => WorkerActivity::Busy,
    JobStatus::Completed | JobStatus::Error | JobStatus::Stopped | JobStatus::Unknown => {
      WorkerActivity::Idle
    }
  };
  let system_info = SystemInformation::new(&worker_configuration);

  ResponseMessage::Feedback(Feedback::Status(ProcessStatus::new(
    WorkerStatus::new(activity, system_info),
    job_result,
  )))
}
