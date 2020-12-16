use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

use crate::message_exchange::{OrderMessage, ResponseMessage};
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

    let _join_handle = std::thread::spawn(move || {
      let process_source: &mut Option<Rc<RefCell<Source>>> = &mut None;
      let process_output: &mut Option<Rc<RefCell<Output>>> = &mut None;

      let mut keep_running = true;

      let mut received = order_receiver.recv();

      while let Ok(message) = &received {
        // Process the received order message
        let response = match message {
          OrderMessage::InitProcess(job) => {
            info!("Initialize job: {:?}", job);

            initialize_process(message_event.clone(), &job)
              .map(|(source, output)| {
                (*process_source) = Some(Rc::new(RefCell::new(source)));
                (*process_output) = Some(Rc::new(RefCell::new(output)));
                ResponseMessage::Initialized
              })
              .unwrap_or_else(ResponseMessage::Error)
          }
          OrderMessage::StartProcess(job) => {
            info!("Process job: {:?}", job);
            if process_source.clone().is_none() || process_output.clone().is_none() {
              ResponseMessage::Error(MessageError::RuntimeError(
                "Process cannot be started, it must be initialized before!".to_string(),
              ))
            } else {
              info!("Start processing job: {:?}", job);

              let job_result = JobResult::from(job);

              let cloned_process_source = process_source.clone().unwrap();
              let cloned_process_output = process_output.clone().unwrap();

              let mut source = cloned_process_source.borrow_mut();
              let mut output = cloned_process_output.borrow_mut();

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
                // Process next frame
                let response = Self::process_frame(
                  message_event.clone(),
                  &mut source,
                  &mut output,
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
                  keep_running = false;
                  break message;
                }

                // Otherwise check whether an order message as been sent to this thread
                if let Ok(message) = order_receiver.try_recv() {
                  let resp = match message {
                    OrderMessage::InitProcess(_) => ResponseMessage::Error(
                      MessageError::RuntimeError("Cannot initialize a running process".to_string()),
                    ),
                    OrderMessage::StartProcess(_) => ResponseMessage::Error(
                      MessageError::RuntimeError("Cannot start a running process".to_string()),
                    ),
                    OrderMessage::StopProcess(_) => {
                      keep_running = false;

                      break finish_process(
                        message_event.clone(),
                        &mut process_output.clone().unwrap().borrow_mut(),
                        JobResult::from(job),
                      )
                      .map(ResponseMessage::Completed)
                      .unwrap_or_else(ResponseMessage::Error);
                    }
                    _ => ResponseMessage::Error(MessageError::RuntimeError(
                      "Cannot handle such a message".to_string(),
                    )),
                  };

                  result_sender.send(resp).unwrap();
                }
              }
            }
          }
          OrderMessage::StopProcess(job) => {
            keep_running = false;
            ResponseMessage::Error(MessageError::RuntimeError(format!(
              "Cannot stop a non-running job: {}",
              job.job_id
            )))
          }
          _ => ResponseMessage::Error(MessageError::RuntimeError(
            "Cannot handle such a message".to_string(),
          )),
        };

        // Send the action response
        result_sender.send(response).unwrap();

        // If the process is stopped, stop looping
        if !keep_running {
          break;
        }

        // Otherwise, wait for the next order message
        received = order_receiver.recv();
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
        self.result_receiver.recv().unwrap_or_else(|error| {
          ResponseMessage::Error(MessageError::RuntimeError(error.to_string()))
        })
      }
      Err(error) => ResponseMessage::Error(MessageError::RuntimeError(error.to_string())),
    }
  }
}

impl MediaProcess {
  fn process_frame<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    message_event: Arc<Mutex<ME>>,
    source: &mut Source,
    output: &mut Output,
    job_result: JobResult,
    response_sender: McaiChannel,
    first_stream_fps: f32,
    process_duration_ms: Option<u64>,
    processed_frames: &mut usize,
    previous_progress: &mut u8,
  ) -> Result<Option<ResponseMessage>> {
    let response = match source.next_frame()? {
      DecodeResult::Frame {
        stream_index,
        frame,
      } => {
        if stream_index == source.get_first_stream_index() {
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
          output,
          job_result,
          stream_index,
          frame,
        )?;
        None
      }
      DecodeResult::WaitMore => None,
      DecodeResult::Nothing => None,
      DecodeResult::EndOfStream => {
        let response = finish_process(message_event, output, job_result)
          .map(ResponseMessage::Completed)?;
        Some(response)
      }
    };
    Ok(response)
  }
}
