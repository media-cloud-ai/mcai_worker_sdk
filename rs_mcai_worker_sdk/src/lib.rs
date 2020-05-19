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
//!
//! #[derive(Debug)]
//! struct WorkerNameEvent {}
//!
//! impl MessageEvent for WorkerNameEvent {
//!   fn get_name(&self) -> String {"sample_worker".to_string()}
//!   fn get_short_description(&self) -> String {"Short description".to_string()}
//!   fn get_description(&self) -> String {"Long description".to_string()}
//!   fn get_version(&self) -> Version { Version::new(0, 0, 1) }
//!   fn get_parameters(&self) -> Vec<Parameter> { vec![] }
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

mod channels;
mod config;
pub mod job;
mod message;
pub mod parameter;
pub mod worker;

/// Re-export from lapin Channel
pub use lapin::Channel;
pub use log::{debug, error, info, trace, warn};
/// Re-export from semver:
pub use semver::Version;

pub use message::publish_job_progression;
pub use parameter::container::ParametersContainer;
pub use parameter::credential::Credential;
pub use parameter::{Parameter, Requirement};

use chrono::prelude::*;
use config::*;
use env_logger::Builder;
use futures_executor::LocalPool;
use futures_util::{future::FutureExt, stream::StreamExt, task::LocalSpawnExt};
use job::{Job, JobResult, JobStatus};
use lapin::{options::*, types::FieldTable, CloseOnDrop, Connection, ConnectionProperties};
use std::{fs, io::Write, sync::Arc, thread, time};

/// Exposed Channel type
pub type McaiChannel = Arc<CloseOnDrop<Channel>>;

/// Trait to describe a worker
///
/// Implement this trait to implement a worker
pub trait MessageEvent {
  fn get_name(&self) -> String;
  fn get_short_description(&self) -> String;
  fn get_description(&self) -> String;
  fn get_version(&self) -> semver::Version;

  fn get_parameters(&self) -> Vec<worker::Parameter>;

  fn process(
    &self,
    _channel: Option<McaiChannel>,
    _job: &Job,
    _job_result: JobResult,
  ) -> Result<JobResult, MessageError>
  where
    Self: std::marker::Sized,
  {
    Err(MessageError::NotImplemented())
  }
}

/// Internal error status to manage process errors
#[derive(Debug, PartialEq)]
pub enum MessageError {
  RuntimeError(String),
  ProcessingError(JobResult),
  RequirementsError(String),
  NotImplemented(),
}

impl MessageError {
  pub fn from(error: std::io::Error, job_result: JobResult) -> Self {
    let result = job_result
      .with_status(JobStatus::Error)
      .with_message(&format!("IO Error: {}", error.to_string()));

    MessageError::ProcessingError(result)
  }
}

/// Function to start a worker
pub fn start_worker<ME: MessageEvent>(message_event: &'static ME)
where
  ME: std::marker::Sync,
{
  let mut builder = Builder::from_default_env();
  let amqp_queue = get_amqp_queue();
  let worker_configuration = worker::WorkerConfiguration::new(&amqp_queue, message_event);

  let container_id = worker_configuration.get_instance_id();

  builder
    .format(move |stream, record| {
      writeln!(
        stream,
        "{} - {} - {} - {} - {} - {}",
        Utc::now(),
        &container_id,
        amqp_queue,
        record.target().parse::<i64>().unwrap_or(-1),
        record.level(),
        record.args(),
      )
    })
    .init();

  let amqp_queue = get_amqp_queue();

  info!(
    "Worker: {}, version: {} (MCAI Worker SDK {})",
    worker_configuration.get_worker_name(),
    worker_configuration.get_worker_version(),
    worker_configuration.get_sdk_version(),
  );

  if let Some(source_orders) = get_source_orders() {
    warn!("Worker will process source orders");
    for source_order in &source_orders {
      info!("Start to process order: {:?}", source_order);

      let count = None;
      let channel = None;
      let message_data = fs::read_to_string(source_order).unwrap();

      let result = message::parse_and_process_message(
        message_event,
        &message_data,
        count,
        channel,
        message::publish_job_progression,
      );

      match result {
        Ok(job_result) => {
          info!(target: &job_result.get_job_id().to_string(), "Succeed process: {:?}", job_result)
        }
        Err(message) => {
          error!("Error {:?}", message);
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
            let delivery = delivery.expect("error caught in in consumer");

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

      consumer
        .for_each(move |delivery| {
          let delivery = delivery.expect("error caught in in consumer");

          message::process_message(message_event, delivery, clone_channel.clone()).map(|_| ())
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

  impl MessageEvent for CustomEvent {
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

    fn get_parameters(&self) -> Vec<worker::Parameter> {
      vec![]
    }
  }

  let custom_event = CustomEvent {};

  let job = job::Job {
    job_id: 1234,
    parameters: vec![],
  };

  let job_result = job::JobResult::new(1234);

  let result = custom_event.process(None, &job, job_result);
  assert!(result == Err(MessageError::NotImplemented()));
}
