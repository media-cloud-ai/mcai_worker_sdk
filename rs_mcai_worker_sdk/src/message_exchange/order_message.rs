use crate::{job::Job, MessageError, Result};
use std::convert::TryFrom;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrderMessage {
  Job(Job),
  InitProcess(Job),
  StartProcess(Job),
  StopProcess(Job),
  StopWorker,
  Status,
}

impl TryFrom<&str> for OrderMessage {
  type Error = MessageError;

  fn try_from(message_data: &str) -> Result<OrderMessage> {
    match serde_json::from_str::<OrderMessage>(message_data) {
      Ok(order_message) => Ok(order_message),
      Err(error) => {
        if let Ok(job_order) = Job::new(message_data) {
          Ok(OrderMessage::Job(job_order))
        } else {
          Err(MessageError::RuntimeError(error.to_string()))
        }
      }
    }
  }
}
