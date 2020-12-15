use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

use crate::message_exchange::{OrderMessage, ResponseMessage};
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
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

pub struct MediaProcess {
  order_sender: Sender<OrderMessage>,
  result_receiver: Receiver<ResponseMessage>,
}

impl<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send> Process<P, ME>
  for MediaProcess
{
  fn new(message_event: Arc<Mutex<ME>>, response_sender: McaiChannel) -> Self {
    let (order_sender, order_receiver) = std::sync::mpsc::channel();
    let (result_sender, result_receiver) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
      let mut process_source: Option<Rc<RefCell<Source>>> = None;
      let mut process_output: Option<Rc<RefCell<Output>>> = None;

      let mut keep_running = true;

      while let Ok(message) = order_receiver.recv() {
        let response = match message {
          OrderMessage::InitProcess(job) => {
            info!("Initialize job: {:?}", job);

            initialize_process(message_event.clone(), &job)
              .map(|(source, output)| {
                process_source = Some(Rc::new(RefCell::new(source)));
                process_output = Some(Rc::new(RefCell::new(output)));
                ResponseMessage::Initialized
              })
              .unwrap_or_else(ResponseMessage::Error)
          }
          OrderMessage::StartProcess(job) => {
            info!("Process job: {:?}", job);
            if process_source.is_none() || process_output.is_none() {
              ResponseMessage::Error(MessageError::RuntimeError(
                "Process cannot be started, it must be initialized before!".to_string(),
              ))
            } else {
              Self::start(
                message_event.clone(),
                &job,
                response_sender.clone(),
                &mut process_source.clone().unwrap().borrow_mut(),
                &mut process_output.clone().unwrap().borrow_mut(),
              )
              .map(ResponseMessage::Completed)
              .unwrap_or_else(ResponseMessage::Error)
            }
          }
          OrderMessage::StopProcess(job) => {
            info!("Stop job: {:?}", job);
            if process_source.is_none() || process_output.is_none() {
              ResponseMessage::Error(MessageError::RuntimeError(
                "Process cannot be started, it must be initialized before!".to_string(),
              ))
            } else {
              keep_running = false;

              finish_process(
                message_event.clone(),
                &mut process_output.clone().unwrap().borrow_mut(),
                JobResult::from(job),
              )
              .map(ResponseMessage::Completed)
              .unwrap_or_else(ResponseMessage::Error)
            }
          }
          _ => ResponseMessage::Error(MessageError::RuntimeError(
            "Cannot handle such a message".to_string(),
          )),
        };
        result_sender.send(response).unwrap();

        if !keep_running {
          break;
        }
      }
    });

    MediaProcess {
      order_sender,
      result_receiver,
    }
  }

  fn handle(
    &mut self,
    _message_event: Arc<Mutex<ME>>,
    order_message: OrderMessage,
  ) -> ResponseMessage {
    match self.order_sender.send(order_message) {
      Ok(()) => {
        return self.result_receiver.recv().unwrap_or_else(|error| {
          ResponseMessage::Error(MessageError::RuntimeError(error.to_string()))
        });
      }
      Err(error) => ResponseMessage::Error(MessageError::RuntimeError(error.to_string())),
    }
  }
}

impl MediaProcess {
  fn start<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    message_event: Arc<Mutex<ME>>,
    job: &Job,
    feedback_sender: McaiChannel,
    source: &mut Source,
    output: &mut Output,
  ) -> Result<JobResult> {
    info!("Start processing job: {:?}", job);

    let job_result = JobResult::from(job);

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
}
