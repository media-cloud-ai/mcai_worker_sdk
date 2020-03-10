#![doc(html_favicon_url = "https://media-io.com/images/mediaio_logo.png")]
#![doc(html_logo_url = "https://media-io.com/images/mediaio_logo.png")]
#![doc(html_no_source)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

mod config;
pub mod job;
mod message;
pub mod parameter;
pub mod worker;

pub use message::{parse_and_process_message, publish_job_progression};
pub use parameter::container::ParametersContainer;
pub use parameter::credential::Credential;
pub use parameter::{Parameter, Requirement};

use amq_protocol_types::AMQPValue;
use amq_protocol_uri::*;
use chrono::prelude::*;
use config::*;
use env_logger::Builder;
use failure::Error;
use futures::{future::Future, Stream};
use job::{Job, JobResult};
use lapin_futures::{
  options::*, types::FieldTable, BasicProperties, Channel, ConnectionProperties, ExchangeKind,
};
use std::{env, fs, io::Write, thread, time};
use tokio::runtime::Runtime;

static EXCHANGE_NAME_SUBMIT: &str = "job_submit";
static EXCHANGE_NAME_RESPONSE: &str = "job_response";
static EXCHANGE_NAME_DELAYED: &str = "job_delayed";
static EXCHANGE_NAME_RESPONSE_DELAYED: &str = "job_response_delayed";

static QUEUE_NAME_WORKER_DISCOVERY: &str = "worker_discovery";

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

fn load_docker_container_id(filename: &str) -> String {
  match fs::read_to_string(filename) {
    Ok(content) => parse_docker_container_id(&content),
    Err(_msg) => "unknown".to_string(),
  }
}

fn parse_docker_container_id(content: &str) -> String {
  let lines: Vec<&str> = content.split('\n').collect();
  if lines.is_empty() {
    return "unknown".to_string();
  }
  let items: Vec<&str> = lines[0].split(':').collect();
  if items.len() != 3 {
    return "unknown".to_string();
  }

  let long_identifier: Vec<&str> = items[2].split("/docker/").collect();
  if long_identifier.len() != 2 {
    return "unknown".to_string();
  }
  let mut identifier = long_identifier[1].to_string();
  identifier.truncate(12);
  identifier
}

#[test]
fn test_load_docker_container_id() {
  assert_eq!(
    load_docker_container_id("./tests/cgroup.sample"),
    "da9002cb1553".to_string()
  );

  assert_eq!(
    load_docker_container_id("/tmp/file_not_exists"),
    "unknown".to_string()
  );

  assert_eq!(parse_docker_container_id(""), "unknown".to_string());
  assert_eq!(parse_docker_container_id("\n"), "unknown".to_string());
  assert_eq!(parse_docker_container_id("a:b:c\n"), "unknown".to_string());
}

pub fn start_worker<ME: MessageEvent>(message_event: &'static ME)
where
  ME: std::marker::Sync,
{
  let mut builder = Builder::from_default_env();
  let container_id = load_docker_container_id("/proc/self/cgroup");
  let queue = get_amqp_queue();

  builder
    .format(move |stream, record| {
      writeln!(
        stream,
        "{} - {} - {} - {} - {} - {}",
        Utc::now(),
        &container_id,
        queue,
        record.target().parse::<i64>().unwrap_or(-1),
        record.level(),
        record.args(),
      )
    })
    .init();

  let queue = get_amqp_queue();
  let version = env::var("VERSION").unwrap_or_else(|_| "unknown".to_string());

  info!("Worker: {}, version: {}", queue, version);

  loop {
    let amqp_tls = get_amqp_tls();
    let amqp_hostname = get_amqp_hostname();
    let amqp_port = get_amqp_port();
    let amqp_username = get_amqp_username();
    let amqp_password = get_amqp_password();
    let amqp_vhost = get_amqp_vhost();
    let amqp_queue = get_amqp_queue();

    info!("Start connection with configuration:");
    info!("AMQP TLS: {}", amqp_tls);
    info!("AMQP HOSTNAME: {}", amqp_hostname);
    info!("AMQP PORT: {}", amqp_port);
    info!("AMQP USERNAME: {}", amqp_username);
    info!("AMQP VHOST: {}", amqp_vhost);
    info!("AMQP QUEUE: {}", amqp_queue);

    let scheme = if amqp_tls {
      AMQPScheme::AMQPS
    } else {
      AMQPScheme::AMQP
    };

    let amqp_uri = AMQPUri {
      scheme,
      authority: AMQPAuthority {
        userinfo: AMQPUserInfo {
          username: amqp_username,
          password: amqp_password,
        },
        host: amqp_hostname,
        port: amqp_port,
      },
      vhost: amqp_vhost,
      query: Default::default(),
    };

    let state = Runtime::new().unwrap().block_on(
      lapin_futures::Client::connect_uri(amqp_uri, ConnectionProperties::default())
        .map_err(Error::from)
        .and_then(|client| client.create_channel().map_err(Error::from))
        .and_then(move |channel| {
          let id = channel.id();
          debug!("created channel with id: {}", id);

          let prefetch_count = 1;
          if let Err(msg) = channel
            .basic_qos(prefetch_count, BasicQosOptions::default())
            .wait()
          {
            error!("Unable to set QoS on channels: {:?}", msg);
          }

          let ch = channel.clone();

          let mut exchange_options = ExchangeDeclareOptions::default();
          exchange_options.durable = true;

          let mut table = FieldTable::default();
          table.insert(
            "alternate-exchange".into(),
            AMQPValue::LongString("job_queue_not_found".into()),
          );
          let mut table_response = FieldTable::default();
          table_response.insert(
            "alternate-exchange".into(),
            AMQPValue::LongString("job_response_not_found".into()),
          );

          if let Err(msg) = channel
            .exchange_declare(
              EXCHANGE_NAME_DELAYED,
              ExchangeKind::Fanout,
              exchange_options.clone(),
              FieldTable::default(),
            )
            .wait()
          {
            error!(
              "Unable to create exchange {}: {:?}",
              EXCHANGE_NAME_DELAYED, msg
            );
          }

          if let Err(msg) = channel
            .exchange_declare(
              EXCHANGE_NAME_SUBMIT,
              ExchangeKind::Topic,
              exchange_options.clone(),
              table,
            )
            .wait()
          {
            error!(
              "Unable to create exchange {}: {:?}",
              EXCHANGE_NAME_SUBMIT, msg
            );
          }

          if let Err(msg) = channel
            .exchange_declare(
              EXCHANGE_NAME_RESPONSE,
              ExchangeKind::Topic,
              exchange_options,
              table_response,
            )
            .wait()
          {
            error!(
              "Unable to create exchange {}: {:?}",
              EXCHANGE_NAME_RESPONSE, msg
            );
          }

          let mut delaying_queue_fields = FieldTable::default();
          delaying_queue_fields.insert(
            "x-dead-letter-exchange".into(),
            AMQPValue::LongString("".into()),
          );
          delaying_queue_fields.insert("x-message-ttl".into(), AMQPValue::ShortInt(5000));

          if let Err(msg) = channel
            .queue_declare(
              &EXCHANGE_NAME_DELAYED,
              QueueDeclareOptions::default(),
              delaying_queue_fields,
            )
            .wait()
          {
            error!(
              "Unable to create queue {}: {:?}",
              EXCHANGE_NAME_DELAYED, msg
            );
          }

          let routing_key = "*";

          if let Err(msg) = channel
            .queue_bind(
              EXCHANGE_NAME_DELAYED,
              EXCHANGE_NAME_DELAYED,
              routing_key,
              QueueBindOptions::default(),
              FieldTable::default(),
            )
            .wait()
          {
            error!("Unable to bind queue {}: {:?}", EXCHANGE_NAME_DELAYED, msg);
          }

          let mut worker_discovery_fields = FieldTable::default();
          worker_discovery_fields.insert(
            "x-dead-letter-exchange".into(),
            AMQPValue::LongString(EXCHANGE_NAME_RESPONSE_DELAYED.into()),
          );
          worker_discovery_fields.insert(
            "x-dead-letter-routing-key".into(),
            AMQPValue::LongString(QUEUE_NAME_WORKER_DISCOVERY.into()),
          );

          channel
            .clone()
            .queue_declare(
              QUEUE_NAME_WORKER_DISCOVERY,
              QueueDeclareOptions {
                durable: true,
                ..Default::default()
              },
              worker_discovery_fields,
            )
            .and_then(|_| {
              let worker_definition =
                worker::WorkerConfiguration::new(&get_amqp_queue(), message_event);

              let msg = json!(worker_definition)
                .to_string()
                .as_str()
                .as_bytes()
                .to_vec();
              channel.basic_publish(
                "",
                QUEUE_NAME_WORKER_DISCOVERY,
                msg,
                BasicPublishOptions::default(),
                BasicProperties::default(),
              )
            })
            .wait()
            .expect("runtime failure");

          let mut queue_fields = FieldTable::default();
          queue_fields.insert(
            "x-dead-letter-exchange".into(),
            AMQPValue::LongString(EXCHANGE_NAME_DELAYED.into()),
          );
          queue_fields.insert(
            "x-dead-letter-routing-key".into(),
            AMQPValue::LongString(amqp_queue.clone().into()),
          );
          queue_fields.insert("x-max-priority".into(), AMQPValue::ShortInt(100));

          channel
            .queue_declare(
              &amqp_queue,
              QueueDeclareOptions {
                durable: true,
                ..Default::default()
              },
              queue_fields,
            )
            .and_then(move |queue| {
              info!("channel {} declared queue {}", id, amqp_queue);

              if let Err(msg) = channel
                .queue_bind(
                  &amqp_queue,
                  EXCHANGE_NAME_SUBMIT,
                  &amqp_queue,
                  QueueBindOptions::default(),
                  FieldTable::default(),
                )
                .wait()
              {
                error!(
                  "Unable to bind queue to exchange {}: {:?}",
                  EXCHANGE_NAME_SUBMIT, msg
                );
              }

              channel.basic_consume(
                &queue,
                "amqp_worker",
                BasicConsumeOptions::default(),
                FieldTable::default(),
              )
            })
            .and_then(move |stream| {
              // process_stream(message_event, channel, stream);
              warn!("start listening stream");

              stream.for_each(move |message| {
                trace!("raw message: {:?}", message);
                message::process_message(message_event, message, &ch);
                Ok(())
              })
            })
            .map_err(Error::from)
        })
        .map_err(Error::from),
    );

    warn!("{:?}", state);
    let sleep_duration = time::Duration::new(1, 0);
    thread::sleep(sleep_duration);
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
