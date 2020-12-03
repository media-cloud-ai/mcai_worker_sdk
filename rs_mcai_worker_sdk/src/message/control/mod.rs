pub mod direct_message;

use std::convert::TryFrom;

use lapin::{message::Delivery, options::BasicPublishOptions, BasicProperties, Channel};

use crate::channels::EXCHANGE_NAME_DIRECT_MESSAGING_RESPONSE;
pub use crate::message::control::direct_message::DirectMessage;
use crate::message::ProcessResponse;
use crate::worker::context::WorkerContext;
use crate::{McaiChannel, MessageError, MessageEvent};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::rc::Rc;

pub fn handle_control_message<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  delivery: Delivery,
  channel: McaiChannel,
  worker_context: &mut WorkerContext,
  message_event: Rc<RefCell<ME>>,
) -> Result<ProcessResponse, MessageError> {
  let direct_message =
    DirectMessage::try_from(&delivery).map_err(|e| MessageError::RuntimeError(e))?;
  direct_message.handle(delivery, channel, worker_context, message_event)
}

fn publish_direct_message_error(
  channel: &Channel,
  _message: &Delivery,
  details: &str,
) -> Result<(), MessageError> {
  error!("An error occurred: {:?}", details);
  let content = json!({
    "status": "error",
    "message": details
  })
  .to_string();

  channel
    .basic_publish(
      EXCHANGE_NAME_DIRECT_MESSAGING_RESPONSE,
      "direct_message_error",
      BasicPublishOptions::default(),
      content.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .map(|_| ())
    .map_err(|e| MessageError::RuntimeError(e.to_string()))

}
