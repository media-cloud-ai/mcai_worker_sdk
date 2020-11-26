use std::convert::TryFrom;

use lapin::{
  message::Delivery,
  options::{BasicAckOptions, BasicPublishOptions, BasicRejectOptions},
  BasicProperties, Channel, Promise,
};

use crate::channels::EXCHANGE_NAME_DIRECT_MESSAGING_RESPONSE;
use crate::worker::{WorkerConfiguration, system_information};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DirectMessage {
  #[serde(rename = "status")]
  Status,
  #[serde(rename = "init")]
  Initialize,
  #[serde(rename = "start")]
  StartProcess,
  #[serde(rename = "stop")]
  StopProcess,
}

impl TryFrom<&Delivery> for DirectMessage {
  type Error = String;

  fn try_from(delivery: &Delivery) -> Result<Self, Self::Error> {
    let message_data = std::str::from_utf8(&delivery.data).map_err(|e| e.to_string())?;

    serde_json::from_str(message_data).map_err(|e| {
      format!(
        "Could not deserialize direct message from {:?}: {}",
        message_data,
        e.to_string()
      )
    })?
  }
}

pub fn handle_control_message(
  delivery: Delivery,
  channel: &Channel,
  worker_configuration: &WorkerConfiguration,
) -> Promise<()> {
  debug!("Handle control message: {:?}", delivery);

  match DirectMessage::try_from(&delivery) {
    Ok(DirectMessage::Status) => {
      system_information::send_real_time_information(delivery, &channel, &worker_configuration)
    }
    Err(error) => publish_direct_message_error(channel, &delivery, &error),
    _ => unimplemented!(),
  }
}

fn publish_direct_message_error(
  channel: &Channel,
  message: &Delivery,
  details: &str,
) -> Promise<()> {
  error!("An error occurred: {:?}", details);
  let content = json!({
    "status": "error",
    "message": details
  })
  .to_string();

  if channel
    .basic_publish(
      EXCHANGE_NAME_DIRECT_MESSAGING_RESPONSE,
      "direct_message_error",
      BasicPublishOptions::default(),
      content.as_bytes().to_vec(),
      BasicProperties::default(),
    )
    .wait()
    .is_ok()
  {
    channel.basic_ack(message.delivery_tag, BasicAckOptions::default())
  } else {
    // NACK and requeue
    channel.basic_reject(message.delivery_tag, BasicRejectOptions { requeue: true })
  }
}

#[test]
pub fn test_direct_message_to_json() {
  let status = DirectMessage::Status;
  let json = serde_json::to_string(&status).unwrap();
  assert_eq!(r#"{"type":"status"}"#, json);
}
