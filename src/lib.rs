#![doc(html_favicon_url = "https://media-io.com/images/mediaio_logo.png")]
#![doc(html_logo_url = "https://media-io.com/images/mediaio_logo.png")]
#![doc(html_no_source)]

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

pub use lapin::Channel;
pub use message::{parse_and_process_message, publish_job_progression};
pub use parameter::container::ParametersContainer;
pub use parameter::credential::Credential;
pub use parameter::{Parameter, Requirement};

use chrono::prelude::*;
use config::*;
use env_logger::Builder;
use futures_executor::LocalPool;
use futures_util::{future::FutureExt, stream::StreamExt, task::LocalSpawnExt};
use job::{Job, JobResult};
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use std::{io::Write, thread, time};

pub trait MessageEvent {
  fn get_name(&self) -> String;
  fn get_short_description(&self) -> String;
  fn get_description(&self) -> String;
  fn get_version(&self) -> semver::Version;

  fn get_parameters(&self) -> Vec<worker::Parameter>;

  fn process(
    &self,
    _channel: Option<&Channel>,
    _job: &Job,
    _job_result: JobResult,
  ) -> Result<JobResult, MessageError>
  where
    Self: std::marker::Sized,
  {
    Err(MessageError::NotImplemented())
  }
}

#[derive(Debug, PartialEq)]
pub enum MessageError {
  RuntimeError(String),
  ProcessingError(JobResult),
  RequirementsError(String),
  NotImplemented(),
}

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
      let channel = channels::declare_consumer_channel(&conn, &worker_configuration);

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

      let status_channel = channel.clone();
      let status_worker_configuration = worker_configuration.clone();

      let _consumer = spawner.spawn_local(async move {
        status_consumer
          .for_each(move |delivery| {
            let delivery = delivery.expect("error caught in in consumer");

            worker::system_information::send_real_time_information(
              delivery,
              &status_channel,
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

          message::process_message(message_event, delivery, &clone_channel).map(|_| ())
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
