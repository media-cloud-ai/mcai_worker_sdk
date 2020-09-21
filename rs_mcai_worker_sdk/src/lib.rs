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

mod channels;
mod config;
mod error;
pub mod job;
mod message;
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
  filters::{AudioFilter, GenericFilter, VideoFilter},
  ttml::{Body, Div, Head, Paragraph, Span, TimeExpression, TimeUnit, Ttml},
  video::{RegionOfInterest, Scaling, VideoFormat},
  StreamDescriptor,
};
pub use message::publish_job_progression;
pub use parameter::container::ParametersContainer;
#[cfg_attr(feature = "cargo-clippy", allow(deprecated))]
pub use parameter::credential::Credential;
pub use parameter::{Parameter, ParameterValue, Requirement};
#[cfg(feature = "media")]
pub use stainless_ffmpeg::{format_context::FormatContext, frame::Frame};

use crate::worker::docker;
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
#[cfg(feature = "media")]
use std::sync::{mpsc::Sender, Mutex};
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

/// # Trait to describe a worker
/// Implement this trait to implement a worker
pub trait MessageEvent<P: DeserializeOwned + JsonSchema> {
  fn get_name(&self) -> String;
  fn get_short_description(&self) -> String;
  fn get_description(&self) -> String;
  fn get_version(&self) -> semver::Version;

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
    _frame: Frame,
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
pub fn start_worker<P: DeserializeOwned + JsonSchema, ME: MessageEvent<P>>(mut message_event: ME)
where
  ME: std::marker::Sync,
{
  let mut builder = Builder::from_default_env();
  let amqp_queue = get_amqp_queue();
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

    return;
  }

  loop {
    let amqp_uri = get_amqp_uri();
    let mut executor = LocalPool::new();
    let spawner = executor.spawner();

    executor.run_until(async {
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

      let consumer = channel
        .clone()
        .basic_consume(
          &amqp_queue,
          "amqp_worker",
          BasicConsumeOptions::default(),
          FieldTable::default(),
        )
        .await
        .unwrap();

      let status_consumer = channel
        .clone()
        .basic_consume(
          &worker_configuration.get_direct_messaging_queue_name(),
          "status_amqp_worker",
          BasicConsumeOptions::default(),
          FieldTable::default(),
        )
        .await
        .unwrap();

      let status_response_channel = channel.clone();
      let status_worker_configuration = worker_configuration.clone();

      let _consumer = spawner.spawn_local(async move {
        status_consumer
          .for_each(move |delivery| {
            let (_channel, delivery) = delivery.expect("error caught in in consumer");

            worker::system_information::send_real_time_information(
              delivery,
              &status_response_channel,
              &status_worker_configuration,
            )
            .map(|_| ())
          })
          .await
      });

      info!("Start to consume on queue {:?}", amqp_queue);

      let clone_channel = channel.clone();
      let message_event = message_event_ref.clone();

      consumer
        .for_each(move |delivery| {
          let (_channel, delivery) = delivery.expect("error caught in in consumer");

          message::process_message(message_event.clone(), delivery, clone_channel.clone())
            .map(|_| ())
        })
        .await
    });

    let sleep_duration = time::Duration::new(1, 0);
    thread::sleep(sleep_duration);
    info!("Reconnection...");
  }
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
