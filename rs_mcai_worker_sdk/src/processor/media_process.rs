use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

use crate::job::{Job, JobProgression, JobStatus};
use crate::message_exchange::{Feedback, OrderMessage, ResponseMessage};
use crate::processor::ProcessStatus;
use crate::worker::status::{WorkerActivity, WorkerStatus};
use crate::worker::system_information::SystemInformation;
use crate::worker::WorkerConfiguration;
use crate::{
  job::JobResult,
  message::media::{
    finish_process, initialize_process,
    output::Output,
    source::{DecodeResult, Source},
  },
  processor::Process,
  publish_job_progression, McaiChannel, MessageError, MessageEvent, Result,
};
use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

pub struct MediaProcess {
  order_sender: Sender<OrderMessage>,
}

impl<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send> Process<P, ME>
  for MediaProcess
{
  fn new(
    message_event: Arc<Mutex<ME>>,
    response_sender: McaiChannel,
    worker_configuration: WorkerConfiguration,
  ) -> Self {
    let (order_sender, order_receiver) = std::sync::mpsc::channel();

    let status = Arc::new(Mutex::new(JobStatus::Unknown));

    let _join_handle = std::thread::spawn(move || {
      let mut process_parameters: Option<Rc<RefCell<MediaProcessParameters>>> = None;

      let mut keep_running = true;

      let mut received = order_receiver.recv();

      while let Ok(message) = &received {
        // Process the received order message
        let response = match message {
          OrderMessage::Job(job) => {
            info!("Process job: {:?}", job);
            let initialization_result =
              MediaProcessParameters::initialize_process(message_event.clone(), job.clone());

            if let Err(error) = initialization_result {
              (*status.lock().unwrap().deref_mut()) = JobStatus::Error;
              ResponseMessage::Error(error)
            } else {
              process_parameters = Some(Rc::new(RefCell::new(initialization_result.unwrap())));

              // TODO send worker response Initialized

              (*status.lock().unwrap().deref_mut()) = JobStatus::Running;
              let response = process_parameters
                .clone()
                .unwrap()
                .borrow_mut()
                .start_process(
                  message_event.clone(),
                  &order_receiver,
                  response_sender.clone(),
                  worker_configuration.clone(),
                );

              if matches!(response, ResponseMessage::Error(_)) {
                (*status.lock().unwrap().deref_mut()) = JobStatus::Error;
              } else {
                (*status.lock().unwrap().deref_mut()) = JobStatus::Completed;
              }

              response
            }
          }
          OrderMessage::InitProcess(job) => {
            let initialization_result =
              MediaProcessParameters::initialize_process(message_event.clone(), job.clone());

            if let Err(error) = initialization_result {
              (*status.lock().unwrap().deref_mut()) = JobStatus::Error;
              ResponseMessage::Error(error)
            } else {
              (*status.lock().unwrap().deref_mut()) = JobStatus::Initialized;
              process_parameters = Some(Rc::new(RefCell::new(initialization_result.unwrap())));

              ResponseMessage::WorkerInitialized(
                JobResult::new(job.job_id).with_status(JobStatus::Initialized),
              )
            }
          }
          OrderMessage::StartProcess(job) => {
            (*status.lock().unwrap().deref_mut()) = JobStatus::Running;

            let response = if let Some(media_process_parameters) = &process_parameters {
              let current_job_id = media_process_parameters.borrow().job.job_id.clone();
              if job.job_id != current_job_id {
                ResponseMessage::Error(MessageError::RuntimeError( // TODO use ProcessError
                  format!("Process cannot be started since another job has been initialized before (id: {})!", current_job_id),
                ))
              } else {
                media_process_parameters.borrow_mut().start_process(
                  message_event.clone(),
                  &order_receiver,
                  response_sender.clone(),
                  worker_configuration.clone(),
                )
              }
            } else {
              ResponseMessage::Error(MessageError::RuntimeError(
                // TODO use ProcessError
                "Process cannot be started, it must be initialized before!".to_string(),
              ))
            };

            info!("Finished response: {:?}", response);

            if matches!(response, ResponseMessage::Error(_)) {
              (*status.lock().unwrap().deref_mut()) = JobStatus::Error;
            } else {
              (*status.lock().unwrap().deref_mut()) = JobStatus::Completed;
            }

            response
          }
          OrderMessage::StopProcess(job) => {
            // TODO check job id is same as store in process_paramaters
            // if job.job_id != process_parameters.unwrap().borrow().job.job_id {
            //
            // }

            ResponseMessage::Error(MessageError::ProcessingError(
              JobResult::new(job.job_id)
                .with_status(JobStatus::Error)
                .with_message("Cannot stop a non-running job."),
            ))
          }
          OrderMessage::Status => Self::get_status_feedback(
            status.lock().unwrap().clone(),
            process_parameters.clone(),
            worker_configuration.clone(),
          ),
          OrderMessage::StopWorker => {
            keep_running = false;
            Self::get_status_feedback(
              status.lock().unwrap().clone(),
              process_parameters.clone(),
              worker_configuration.clone(),
            )
          }
        };

        // Send the action response
        trace!("Send the action response message...");
        response_sender
          .lock()
          .unwrap()
          .send_response(response)
          .unwrap();

        // If the process is stopped, stop looping
        if !keep_running {
          break;
        }

        // Otherwise, wait for the next order message
        received = order_receiver.recv();
      }
    });

    MediaProcess { order_sender }
  }

  fn handle(&mut self, _message_event: Arc<Mutex<ME>>, order_message: OrderMessage) -> Result<()> {
    if let Err(error) = self.order_sender.send(order_message) {
      return Err(MessageError::RuntimeError(error.to_string())); // TODO use ProcessError
    }
    Ok(())
  }
}

impl MediaProcess {
  fn get_status_feedback(
    status: JobStatus,
    process_parameters: Option<Rc<RefCell<MediaProcessParameters>>>,
    worker_configuration: WorkerConfiguration,
  ) -> ResponseMessage {
    let job_result = process_parameters
      .map(|param| JobResult::new(param.borrow().job.job_id).with_status(status.clone()));

    get_status_feedback(status, job_result, worker_configuration)
  }
}

fn get_status_feedback(
  status: JobStatus,
  job_result: Option<JobResult>,
  worker_configuration: WorkerConfiguration,
) -> ResponseMessage {
  let activity = match &status {
    JobStatus::Initialized | JobStatus::Running => WorkerActivity::Busy,
    JobStatus::Completed | JobStatus::Error | JobStatus::Unknown => WorkerActivity::Idle,
  };
  let system_info = SystemInformation::new(&worker_configuration);

  ResponseMessage::Feedback(Feedback::Status(ProcessStatus::new(
    WorkerStatus::new(activity, system_info),
    job_result,
  )))
}

pub struct MediaProcessParameters {
  source: Source,
  output: Output,
  keep_running: bool,
  job: Job,
}

impl MediaProcessParameters {
  fn initialize_process<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    message_event: Arc<Mutex<ME>>,
    job: Job,
  ) -> Result<Self> {
    info!("Initialize job: {:?}", job);

    initialize_process(message_event, &job).map(|(source, output)| MediaProcessParameters {
      source,
      output,
      keep_running: false,
      job,
    })
  }

  fn get_status_feedback(
    &self,
    status: JobStatus,
    worker_configuration: WorkerConfiguration,
  ) -> ResponseMessage {
    let job_result = JobResult::new(self.job.job_id).with_status(status.clone());

    get_status_feedback(status, Some(job_result), worker_configuration)
  }

  fn start_process<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    order_receiver: &Receiver<OrderMessage>,
    response_sender: McaiChannel,
    worker_configuration: WorkerConfiguration,
  ) -> ResponseMessage {
    let job = self.job.clone();

    info!("Start processing job: {:?}", job);

    // start publishing progression
    response_sender
      .lock()
      .unwrap()
      .send_response(ResponseMessage::Feedback(Feedback::Progression(
        JobProgression::new(job.job_id, 0),
      )))
      .unwrap();

    let job_result = JobResult::from(job.clone());

    info!(
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

  fn process_frame<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
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
        info!(
          "{} - Process frame {}",
          job_result.get_str_job_id(),
          processed_frames
        );

        crate::message::media::process_frame(
          message_event,
          &mut self.output,
          job_result,
          stream_index,
          frame,
        )?;
        None
      }
      DecodeResult::WaitMore => None,
      DecodeResult::Nothing => None,
      DecodeResult::EndOfStream => {
        let response = finish_process(message_event, &mut self.output, job_result)
          .map(ResponseMessage::Completed)?;
        Some(response)
      }
    };

    Ok(response)
  }
}
