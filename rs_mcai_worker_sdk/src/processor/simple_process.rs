use crate::message_exchange::{OrderMessage, ResponseMessage};
use crate::{job::JobResult, processor::Process, McaiChannel, MessageError, MessageEvent};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

pub struct SimpleProcess {
  response_sender: McaiChannel,
}

impl<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send> Process<P, ME>
  for SimpleProcess
{
  fn new(_message_event: Arc<Mutex<ME>>, response_sender: McaiChannel) -> Self {
    SimpleProcess { response_sender }
  }

  fn handle(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    order_message: OrderMessage,
  ) -> ResponseMessage {
    match order_message {
      OrderMessage::InitProcess(_job) => ResponseMessage::Initialized,
      OrderMessage::StartProcess(job) => {
        info!("Process job: {:?}", job);
        message_event
          .lock()
          .unwrap()
          .process(
            Some(self.response_sender.clone()),
            job.get_parameters().unwrap(),
            JobResult::from(job),
          )
          .map(ResponseMessage::Completed)
          .unwrap_or_else(ResponseMessage::Error)
      }
      OrderMessage::StopProcess(job) => ResponseMessage::Completed(JobResult::from(job)),
      _ => ResponseMessage::Error(MessageError::RuntimeError(
        "Cannot handle such a message".to_string(),
      )),
    }
  }
}
