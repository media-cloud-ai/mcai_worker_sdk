use std::{cell::RefCell, convert::TryFrom, rc::Rc};

use chrono::{DateTime, Utc};
use lapin::message::Delivery;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

#[cfg(feature = "media")]
use crate::job::JobResult;
use crate::worker::context::WorkerContext;
#[cfg(feature = "media")]
use crate::{
  message::media::{initialize_process, launch_process, stop_process},
  worker::context::WorkerStatus,
};

use crate::message::ProcessResponse;
use crate::{job::Job, worker::system_information, McaiChannel, MessageError, MessageEvent};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DirectMessage {
  #[serde(rename = "status")]
  Status,
  #[serde(rename = "init")]
  Initialize(Job),
  #[serde(rename = "start")]
  StartProcess {
    job: Job,
    scheduled: Option<DateTime<Utc>>,
  },
  #[serde(rename = "stop")]
  StopProcess {
    job: Job,
    scheduled: Option<DateTime<Utc>>,
  },
}

impl TryFrom<&Delivery> for DirectMessage {
  type Error = String;

  // TODO from ProcessRequest
  fn try_from(delivery: &Delivery) -> Result<Self, Self::Error> {
    let message_data = std::str::from_utf8(&delivery.data).map_err(|e| e.to_string())?;

    serde_json::from_str(message_data).map_err(|e| {
      format!(
        "Could not deserialize direct message from {:?}: {}",
        message_data,
        e.to_string()
      )
    })
  }
}

impl DirectMessage {
  #[cfg(feature = "media")]
  pub fn handle<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    &self,
    delivery: Delivery,
    channel: McaiChannel,
    worker_context: &mut WorkerContext,
    message_event: Rc<RefCell<ME>>,
  ) -> Result<ProcessResponse, MessageError> {
    match &self {
      DirectMessage::Status => {
        if let Some(worker_configuration) = &worker_context.configuration {
          system_information::send_real_time_information(
            delivery.clone(),
            channel,
            worker_configuration,
          )?;
        } else {
          error!("No worker configuration!");
          unhandle_message(delivery.clone(), &format!("{:?}", self))?;
        }
        Ok(ProcessResponse::new(delivery))
      }
      DirectMessage::Initialize(job) => {
        handle_initialize_message(delivery, message_event, channel, job, worker_context)
      }
      DirectMessage::StartProcess { job, scheduled } => handle_start_process_message(
        delivery,
        message_event,
        channel,
        job,
        worker_context,
        scheduled,
      ),
      DirectMessage::StopProcess { job, scheduled } => {
        handle_stop_process_message(delivery, message_event, job, worker_context, scheduled)
      }
    }
  }

  #[cfg(not(feature = "media"))]
  pub fn handle<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
    &self,
    delivery: Delivery,
    channel: McaiChannel,
    worker_context: &mut WorkerContext,
    _message_event: Rc<RefCell<ME>>,
  ) -> Result<ProcessResponse, MessageError> {
    match &self {
      DirectMessage::Status => {
        if let Some(worker_configuration) = &worker_context.configuration {
          system_information::send_real_time_information(
            delivery.clone(),
            channel,
            worker_configuration,
          )?;
        } else {
          error!("No worker configuration!");
          unhandle_message(delivery.clone(), &format!("{:?}", self))?;
        }
      }
      other => unhandle_message(delivery.clone(), &format!("{:?}", other))?,
    }
    Ok(ProcessResponse::new(delivery))
  }
}

#[cfg(feature = "media")]
fn handle_initialize_message<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  delivery: Delivery,
  message_event: Rc<RefCell<ME>>,
  channel: McaiChannel,
  job: &Job,
  worker_context: &mut WorkerContext,
) -> Result<ProcessResponse, MessageError> {
  // direct_thread_sender.try_send(format!("Init bitch!")).unwrap();

  initialize_process(message_event, Some(channel.clone()), worker_context, job)?;
  worker_context.status = WorkerStatus::INITIALIZED;
  Ok(ProcessResponse::new(delivery.clone()))
}

#[cfg(feature = "media")]
fn handle_start_process_message<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  delivery: Delivery,
  message_event: Rc<RefCell<ME>>,
  channel: McaiChannel,
  job: &Job,
  worker_context: &mut WorkerContext,
  _scheduled: &Option<DateTime<Utc>>,
) -> Result<ProcessResponse, MessageError> {
  // TODO:
  //  - move to process thread
  //  - based on UTC time or media timestamp
  // if let Some(date_time) = scheduled {
  //   let now = DateTime::<Utc>::from(Local::now());
  //   let delay_ms = date_time.timestamp_millis() - now.timestamp_millis();
  //   std::thread::sleep(Duration::from_millis(delay_ms as u64));
  // }

  worker_context.status = WorkerStatus::RUNNING;
  let job_result = launch_process(
    message_event,
    Some(channel.clone()),
    worker_context,
    job,
    JobResult::new(job.job_id),
  )?;

  Ok(ProcessResponse {
    delivery: delivery.clone(),
    job_result: Some(job_result),
    error: None,
  })
}

#[cfg(feature = "media")]
fn handle_stop_process_message<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(
  delivery: Delivery,
  message_event: Rc<RefCell<ME>>,
  job: &Job,
  worker_context: &mut WorkerContext,
  _scheduled: &Option<DateTime<Utc>>,
) -> Result<ProcessResponse, MessageError> {
  let job_result = stop_process(message_event, worker_context, JobResult::new(job.job_id))?;
  worker_context.status = WorkerStatus::STOPPED;

  Ok(ProcessResponse {
    delivery: delivery.clone(),
    job_result: Some(job_result),
    error: None,
  })
}

fn unhandle_message(delivery: Delivery, message_type: &str) -> Result<(), MessageError> {
  warn!(
    "Non-media workers do not handle such a '{}' direct message: {:?}",
    message_type, delivery
  );
  Ok(())
  // channel.basic_reject(delivery.delivery_tag, BasicRejectOptions { requeue: false })
}

#[test]
pub fn test_direct_message_to_json() {
  use chrono::Local;

  let status = DirectMessage::Status;
  let json = serde_json::to_string(&status).unwrap();
  assert_eq!(r#"{"type":"status"}"#, json);

  let message = r#"{
    "job_id": 123,
    "parameters": [
      {
        "id":"string_parameter",
        "type":"string",
        "value":"hello"
      }
    ]
  }"#;
  let job = Job::new(message).unwrap();
  let init = DirectMessage::Initialize(job.clone());
  let json = serde_json::to_string(&init).unwrap();
  assert_eq!(
    r#"{"type":"init","job_id":123,"parameters":[{"id":"string_parameter","type":"string","store":null,"value":"hello","default":null}]}"#,
    json
  );

  let start = DirectMessage::StartProcess {
    job: job.clone(),
    scheduled: None,
  };
  let json = serde_json::to_string(&start).unwrap();
  assert_eq!(
    r#"{"type":"start","job":{"job_id":123,"parameters":[{"id":"string_parameter","type":"string","store":null,"value":"hello","default":null}]},"scheduled":null}"#,
    json
  );

  let date_time: DateTime<Utc> = DateTime::from(Local::now());
  let date_time_string = serde_json::to_string(&date_time).unwrap();

  let scheduled_start = DirectMessage::StartProcess {
    job: job.clone(),
    scheduled: Some(date_time),
  };
  let json = serde_json::to_string(&scheduled_start).unwrap();
  assert_eq!(
    format!(
      "{}{}{}",
      r#"{"type":"start","job":{"job_id":123,"parameters":[{"id":"string_parameter","type":"string","store":null,"value":"hello","default":null}]},"scheduled":"#,
      date_time_string,
      r#"}"#
    ),
    json
  );
}
