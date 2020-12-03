//! # MCAI Worker SDK
//!
//! This library is an SDK to communicate via message broker with [StepFlow](https://hexdocs.pm/step_flow/readme.html).
//! It's used for every worker as an abstraction.
//! It manage itself requirements, message parsing, direct messaging.
//!
//! ## Worker implementation
//!
//! 1. Create a Rust project
//! 2. Add MCAI Worker SDK as a dependency in Cargo.toml: `mcai_worker_sdk = "^1.0"`
//! 1. Update the main file with the example provided here to implement [MessageEvent](trait.MessageEvent.html) trait,
//! and call the [`start_worker`](fn.start_worker.html) to start the worker itself.
//!
//! ```rust
//! use mcai_worker_sdk::{
//!   MessageEvent,
//!   Version,
//!   worker::Parameter,
//! };
//! use serde_derive::Deserialize;
//! use schemars::JsonSchema;
//!
//! #[derive(Debug)]
//! struct WorkerNameEvent {}
//!
//! #[derive(Debug, Deserialize, JsonSchema)]
//! struct WorkerParameters {}
//!
//! impl MessageEvent<WorkerParameters> for WorkerNameEvent {
//!   fn get_name(&self) -> String {"sample_worker".to_string()}
//!   fn get_short_description(&self) -> String {"Short description".to_string()}
//!   fn get_description(&self) -> String {"Long description".to_string()}
//!   fn get_version(&self) -> Version { Version::new(0, 0, 1) }
//! }
//! static WORKER_NAME_EVENT: WorkerNameEvent = WorkerNameEvent {};
//!
//! // uncomment it to start the worker
//! // fn main() {
//! //   mcai_worker_sdk::start_worker(&WORKER_NAME_EVENT);
//! // }
//! ```
//!
//! ## Runtime configuration
//!
//! ### AMQP connection
//!
//! |    Variable     | Description |
//! |-----------------|-------------|
//! | `AMQP_HOSTNAME` | IP or host of AMQP server (default: `localhost`) |
//! | `AMQP_PORT`     | AMQP server port (default: `5672`) |
//! | `AMQP_TLS`      | enable secure connection using AMQPS (default: `false`, enable with `true` or `1` or `TRUE` or `True`) |
//! | `AMQP_USERNAME` | Username used to connect to AMQP server (default: `guest`) |
//! | `AMQP_PASSWORD` | Password used to connect to AMQP server (default: `guest`) |
//! | `AMQP_VHOST`    | AMQP virtual host (default: `/`) |
//! | `AMQP_QUEUE`    | AMQP queue name used to receive job orders (default: `job_undefined`) |
//!
//! ### Vault connection
//!
//! |    Variable        | Description |
//! |--------------------|-------------|
//! | `BACKEND_HOSTNAME` | URL used to connect to backend server (default: `http://127.0.0.1:4000/api`) |
//! | `BACKEND_USERNAME` | Username used to connect to backend server |
//! | `BACKEND_PASSWORD` | Password used to connect to backend server |
//!
//! ## Start worker locally
//!
//! MCAI Worker SDK can be launched locally - without RabbitMQ.
//! It can process some message for different purpose (functional tests, message order examples, etc.).
//!
//! To start worker in this mode, setup the environment variable `SOURCE_ORDERS` with path(s) to json orders.
//! It can take multiple orders, joined with `:` on unix platform, `;` on windows os.
//!
//! ### Examples:
//!
//! ```bash
//! RUST_LOG=info SOURCE_ORDERS=./examples/success_order.json:./examples/error_order.json cargo run --example worker
//! ```

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[cfg(feature = "media")]
#[macro_use]
extern crate yaserde_derive;

pub mod channels;
mod config;
mod error;
pub mod job;
pub mod message;
pub mod parameter;
pub mod worker;

/// Re-export from lapin Channel
pub use lapin::Channel;
pub use log::{debug, error, info, trace, warn};
pub use schemars::JsonSchema;
/// Re-export from semver:
pub use semver::Version;

pub use error::{MessageError, Result};
#[cfg(feature = "media")]
pub use message::media::{
  audio::AudioFormat,
  ebu_ttml_live::{
    Body, Div, EbuTtmlLive, Frames, Head, Paragraph, Span, Styling, TimeExpression, TimeUnit, Title,
  },
  filters::{AudioFilter, GenericFilter, VideoFilter},
  output::Output,
  source::Source,
  video::{RegionOfInterest, Scaling, VideoFormat},
  StreamDescriptor,
};
pub use message::publish_job_progression;
pub use parameter::container::ParametersContainer;
pub use parameter::{Parameter, ParameterValue, Requirement};
#[cfg(feature = "media")]
pub use stainless_ffmpeg::{format_context::FormatContext, frame::Frame};

use crate::worker::context::WorkerContext;
use crate::worker::{docker, WorkerConfiguration};

use chrono::prelude::*;
use config::*;
use env_logger::Builder;
use futures_executor::LocalPool;
use futures_util::{future::FutureExt, stream::StreamExt, task::LocalSpawnExt};
use job::JobResult;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use serde::de::DeserializeOwned;
#[cfg(feature = "media")]
use serde::Serialize;
use std::str::FromStr;
// #[cfg(feature = "media")]
use crate::message::{
  publish_job_completed, publish_missing_requirements, publish_not_implemented,
  publish_parameter_error, publish_processing_error, publish_runtime_error, ProcessRequest,
  ProcessResponse,
};
use std::sync::mpsc::{Receiver, Sender};
#[cfg(feature = "media")]
use std::sync::Mutex;
use std::{cell::RefCell, fs, io::Write, rc::Rc, sync::Arc, thread, time};
#[cfg(feature = "media")]
use yaserde::YaSerialize;

/// Exposed Channel type
pub type McaiChannel = Arc<Channel>;

#[cfg(feature = "media")]
#[derive(Debug)]
pub struct ProcessResult {
  end_of_process: bool,
  json_content: Option<String>,
  xml_content: Option<String>,
}

#[cfg(feature = "media")]
impl ProcessResult {
  pub fn empty() -> Self {
    ProcessResult {
      end_of_process: false,
      json_content: None,
      xml_content: None,
    }
  }

  pub fn end_of_process() -> Self {
    ProcessResult {
      end_of_process: true,
      json_content: None,
      xml_content: None,
    }
  }

  pub fn new_json<S: Serialize>(content: S) -> Self {
    let content = serde_json::to_string(&content).unwrap();

    ProcessResult {
      end_of_process: false,
      json_content: Some(content),
      xml_content: None,
    }
  }

  pub fn new_xml<Y: YaSerialize>(content: Y) -> Self {
    let content = yaserde::ser::to_string(&content).unwrap();

    ProcessResult {
      end_of_process: false,
      json_content: None,
      xml_content: Some(content),
    }
  }
}

#[cfg(feature = "media")]
pub enum ProcessFrame {
  AudioVideo(Frame),
  EbuTtmlLive(Box<EbuTtmlLive>),
  Data(Vec<u8>),
}

#[cfg(feature = "media")]
impl ProcessFrame {
  pub fn get_pts(&self) -> i64 {
    match self {
      ProcessFrame::AudioVideo(frame) => frame.get_pts(),
      ProcessFrame::EbuTtmlLive(_) | ProcessFrame::Data(_) => {
        // improvement: support pts to terminate
        0
      }
    }
  }
}

/// # Trait to describe a worker
/// Implement this trait to implement a worker
pub trait MessageEvent<P: DeserializeOwned + JsonSchema> {
  fn get_name(&self) -> String;
  fn get_short_description(&self) -> String;
  fn get_description(&self) -> String;
  fn get_version(&self) -> semver::Version;

  /// Initialize worker context.
  /// Override it for specific usage.
  fn init(&mut self) -> Result<()> {
    Ok(())
  }

  #[cfg(feature = "media")]
  fn init_process(
    &mut self,
    _parameters: P,
    _format_context: Arc<Mutex<FormatContext>>,
    _response_sender: Arc<Mutex<Sender<ProcessResult>>>,
  ) -> Result<Vec<StreamDescriptor>> {
    Ok(vec![])
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &mut self,
    _job_result: JobResult,
    _stream_index: usize,
    _frame: ProcessFrame,
  ) -> Result<ProcessResult> {
    Err(MessageError::NotImplemented())
  }

  #[cfg(feature = "media")]
  fn ending_process(&mut self) -> Result<()> {
    Ok(())
  }

  /// Not called when the "media" feature is enabled
  fn process(
    &self,
    _channel: Option<McaiChannel>,
    _parameters: P,
    _job_result: JobResult,
  ) -> Result<JobResult>
  where
    Self: std::marker::Sized,
  {
    Err(MessageError::NotImplemented())
  }
}

/// Function to start a worker
pub fn start_worker<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P>>(mut message_event: ME)
where
  ME: std::marker::Sync,
{
  let mut builder = Builder::from_default_env();
  let instance_id = docker::get_instance_id("/proc/self/cgroup");
  let container_id = instance_id.clone();
  builder
    .format(move |stream, record| {
      writeln!(
        stream,
        "{} - {} - {} - {} - {} - {}",
        Utc::now(),
        &container_id,
        get_amqp_queue(),
        record.target().parse::<i64>().unwrap_or(-1),
        record.level(),
        record.args(),
      )
    })
    .init();

  let amqp_queue = get_amqp_queue();
  let worker_configuration =
    worker::WorkerConfiguration::new(&amqp_queue, &message_event, &instance_id);
  if let Err(configuration_error) = worker_configuration {
    error!("{:?}", configuration_error);
    return;
  }

  let worker_configuration = worker_configuration.unwrap();

  info!(
    "Worker: {}, version: {} (MCAI Worker SDK {})",
    worker_configuration.get_worker_name(),
    worker_configuration.get_worker_version(),
    worker_configuration.get_sdk_version(),
  );

  if let Ok(enabled) = std::env::var("DESCRIBE") {
    if enabled == "1" || bool::from_str(&enabled.to_lowercase()).unwrap_or(false) {
      match serde_json::to_string_pretty(&worker_configuration) {
        Ok(serialized_configuration) => {
          println!("{}", serialized_configuration);
          return;
        }
        Err(error) => error!("Could not serialize worker configuration: {:?}", error),
      }
    }
  }

  if let Err(message) = message_event.init() {
    error!("{:?}", message);
    return;
  }

  let message_event_ref = Rc::new(RefCell::new(message_event));

  info!("Worker initialized, ready to receive jobs");

  if let Some(source_orders) = get_source_orders() {
    warn!("Worker will process source orders");
    return start_source_orders_process(source_orders, message_event_ref);
  }

  let (message_thread_sender, process_thread_receiver) = std::sync::mpsc::channel();
  let (process_thread_sender, message_thread_receiver) = std::sync::mpsc::channel(); // async_channel::unbounded();

  start_consumers_thread(
    message_thread_sender,
    message_thread_receiver,
    worker_configuration.clone(),
  );

  let mut worker_context = WorkerContext::new(Some(worker_configuration.clone()));
  let cloned_message_event = message_event_ref.clone();

  let message_event = message_event_ref.clone();

  loop {
    if let Ok(process_request) = process_thread_receiver.recv() {
      info!("Process thread received: {:?}", process_request);
      let delivery = process_request.delivery.clone();
      let result = match delivery.exchange.as_str() {
        "direct_messaging" => message::control::handle_control_message(
          delivery.clone(),
          process_request.channel.clone(),
          &mut worker_context,
          cloned_message_event.clone(),
        ),
        _ => message::process_message(
          message_event.clone(),
          delivery.clone(),
          process_request.channel.clone(),
        ),
      };

      let response = match result {
        Ok(response) => response,
        Err(error) => ProcessResponse::new_with_error(delivery, error),
      };

      if let Err(error) = process_thread_sender.send(response) {
        error!("Could not send process response: {}", error.to_string());
      }
    }
  }
}

fn start_source_orders_process<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P>>(
  source_orders: Vec<String>,
  message_event_ref: Rc<RefCell<ME>>,
) {
  for source_order in &source_orders {
    info!("Start to process order: {:?}", source_order);

    let count = None;
    let channel = None;
    let message_data = fs::read_to_string(source_order).unwrap();

    let result = message::parse_and_process_message(
      message_event_ref.clone(),
      &message_data,
      count,
      channel,
      message::publish_job_progression,
    );

    match result {
      Ok(mut job_result) => {
        job_result.update_execution_duration();
        info!(target: &job_result.get_job_id().to_string(), "Process succeeded: {:?}", job_result)
      }
      Err(message) => {
        error!("{:?}", message);
      }
    }
  }
}

fn start_consumers_thread<'a>(
  message_thread_sender: Sender<ProcessRequest>,
  message_thread_receiver: Receiver<ProcessResponse>,
  worker_configuration: WorkerConfiguration,
) {
  std::thread::spawn(move || {
    loop {
      let amqp_uri = get_amqp_uri();
      let mut executor = LocalPool::new();
      let spawner = executor.spawner();

      let conn = Connection::connect_uri(
        amqp_uri,
        ConnectionProperties::default().with_default_executor(8),
      )
      .wait()
      .unwrap();

      info!("Connected");
      let channel = Arc::new(channels::declare_consumer_channel(
        &conn,
        &worker_configuration,
      ));

      let message_response_channel = channel.clone();

      let _result = spawner.spawn_local(async move {
        while let Ok(response) = message_thread_receiver.recv() {
          match response {
            ProcessResponse {
              delivery,
              error: Some(error),
              ..
            } => match error {
              MessageError::RequirementsError(details) => {
                publish_missing_requirements(message_response_channel.clone(), delivery, &details);
              }
              MessageError::NotImplemented() => {
                publish_not_implemented(message_response_channel.clone(), delivery);
              }
              MessageError::ParameterValueError(error_message) => {
                publish_parameter_error(message_response_channel.clone(), delivery, &error_message);
              }
              MessageError::ProcessingError(job_result) => {
                publish_processing_error(message_response_channel.clone(), delivery, job_result);
              }
              MessageError::RuntimeError(error_message) => {
                publish_runtime_error(message_response_channel.clone(), delivery, &error_message);
              }
            },
            ProcessResponse {
              delivery,
              job_result: Some(job_result),
              ..
            } => {
              info!(target: &job_result.get_str_job_id(), "Completed");
              publish_job_completed(message_response_channel.clone(), delivery, job_result);
            }
            _ => {
              // Nothing to do
            }
          };
        }
      });

      executor.run_until(async {
        let message_consumer = channel
          .clone()
          .basic_consume(
            &worker_configuration.get_queue_name(),
            "amqp_worker",
            BasicConsumeOptions::default(),
            FieldTable::default(),
          )
          .await
          .unwrap();

        let direct_message_queue_name = worker_configuration.get_direct_messaging_queue_name();
        let direct_message_consumer = channel
          .clone()
          .basic_consume(
            &direct_message_queue_name.clone(),
            "status_amqp_worker",
            BasicConsumeOptions::default(),
            FieldTable::default(),
          )
          .await
          .unwrap();

        let direct_message_response_channel = channel.clone();
        let direct_thread_sender = message_thread_sender.clone();

        let _direct_consumer_result = spawner.spawn_local(async move {
          info!(
            "Start consuming direct message queue: {}",
            direct_message_queue_name
          );
          direct_message_consumer
            .for_each(move |delivery| {
              let (channel, delivery) = delivery.expect("error caught in in consumer");

              info!("Handle direct delivery: {:?}", delivery);

              let delivery_tag = delivery.delivery_tag.clone();

              if direct_thread_sender
                .send(ProcessRequest {
                  delivery,
                  channel: Arc::new(channel),
                })
                .is_ok()
              {
                info!("ACK direct delivery reception...");
                direct_message_response_channel.basic_ack(delivery_tag, BasicAckOptions::default())
              } else {
                info!("Reject direct delivery reception...");
                direct_message_response_channel
                  .basic_reject(delivery_tag, BasicRejectOptions { requeue: false })
              }
              .map(|_| ())
            })
            .await
        });

        info!(
          "Start to consume on queue {:?}",
          worker_configuration.get_queue_name()
        );

        let cloned_message_thread_sender = message_thread_sender.clone();
        let message_response_channel = channel.clone();

        message_consumer
          .for_each(move |delivery| {
            let (channel, delivery) = delivery.expect("error caught in in consumer");

            let delivery_tag = delivery.delivery_tag.clone();
            if cloned_message_thread_sender
              .send(ProcessRequest {
                delivery,
                channel: Arc::new(channel),
              })
              .is_ok()
            {
              message_response_channel.basic_ack(delivery_tag, BasicAckOptions::default())
            } else {
              message_response_channel
                .basic_reject(delivery_tag, BasicRejectOptions { requeue: false })
            }
            .map(|_| ())

            // promise.map(|_| ())
          })
          .await
      });

      let sleep_duration = time::Duration::new(1, 0);
      thread::sleep(sleep_duration);
      info!("Reconnection...");
    }
  });
}

#[test]
fn empty_message_event_impl() {
  #[derive(Debug)]
  struct CustomEvent {}

  #[derive(JsonSchema, Deserialize)]
  struct CustomParameters {}

  impl MessageEvent<CustomParameters> for CustomEvent {
    fn get_name(&self) -> String {
      "custom".to_string()
    }
    fn get_short_description(&self) -> String {
      "short description".to_string()
    }
    fn get_description(&self) -> String {
      "long description".to_string()
    }
    fn get_version(&self) -> semver::Version {
      semver::Version::new(1, 2, 3)
    }
  }

  let custom_event = CustomEvent {};
  let parameters = CustomParameters {};

  let job = job::Job {
    job_id: 1234,
    parameters: vec![],
  };

  let job_result = job::JobResult::new(job.job_id);

  let result = custom_event.process(None, parameters, job_result);
  assert!(result == Err(MessageError::NotImplemented()));
}
