#![doc(html_favicon_url = "https://media-io.com/images/mediaio_logo.png")]
#![doc(html_logo_url = "https://media-io.com/images/mediaio_logo.png")]
#![doc(html_no_source)]

extern crate amq_protocol_types;
extern crate amq_protocol_uri;
extern crate chrono;
extern crate env_logger;
extern crate failure;
extern crate futures;
#[macro_use]
extern crate log;
extern crate lapin_futures as lapin;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate tokio;

mod config;
pub mod job;

use amq_protocol_types::AMQPValue;
use amq_protocol_uri::*;
use chrono::prelude::*;
use config::*;
use env_logger::Builder;
use failure::Error;
use futures::future::Future;
use futures::Stream;
use job::{JobResult, JobStatus};
use lapin::options::*;
use lapin::types::FieldTable;
use lapin::{BasicProperties, ConnectionProperties};
use std::{
  env,
  fs,
  io::Write,
  thread,
  time,
};
use tokio::runtime::Runtime;

pub trait MessageEvent {
  fn process(&self, _message: &str) -> Result<JobResult, MessageError>
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
    Ok(content) => {
      let lines: Vec<&str> = content.split("\n").collect();
      if lines.is_empty() {
        return "unknown".to_string();
      }
      let items: Vec<&str> = lines[0].split(":").collect();
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
    Err(_msg) => "unknown".to_string(),
  }
}

#[test]
fn test_load_docker_container_id() {
  assert_eq!(
    load_docker_container_id("./tests/cgroup.sample"),
    "da9002cb1553".to_string()
  );
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
        "{} - {} - {} - {} - {}",
        Utc::now(),
        &container_id,
        queue,
        record.level(),
        record.args()
      )
    })
    .init();

  loop {
    let amqp_tls = get_amqp_tls();
    let amqp_hostname = get_amqp_hostname();
    let amqp_port = get_amqp_port();
    let amqp_username = get_amqp_username();
    let amqp_password = get_amqp_password();
    let amqp_vhost = get_amqp_vhost();
    let amqp_queue = get_amqp_queue();
    let amqp_completed_queue = get_amqp_completed_queue();
    let amqp_error_queue = get_amqp_error_queue();

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

    let state = Runtime::new().unwrap().block_on_all(
      lapin::Client::connect_uri(amqp_uri, ConnectionProperties::default())
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

          let delayed_name = amqp_queue.clone() + "_delayed";
          let exchange_type = "fanout";

          if let Err(msg) = channel
            .exchange_declare(
              &delayed_name,
              exchange_type,
              ExchangeDeclareOptions::default(),
              FieldTable::default(),
            ).wait()
          {
            error!("Unable to create exchange {}: {:?}", delayed_name, msg);
          }

          let mut fields_delayed = FieldTable::default();
          fields_delayed.insert("x-dead-letter-exchange".into(), AMQPValue::LongString("".into()));
          fields_delayed.insert("x-message-ttl".into(), AMQPValue::ShortInt(5000));

          if let Err(msg) = channel.queue_declare(
            &delayed_name,
            QueueDeclareOptions::default(),
            fields_delayed,
          ).wait() {
            error!("Unable to create queue {}: {:?}", delayed_name, msg);
          }

          let routing_key = "*";

          if let Err(msg) = channel.queue_bind(
            &delayed_name,
            &delayed_name,
            routing_key,
            QueueBindOptions::default(),
            FieldTable::default()
          ).wait() {
            error!("Unable to create queue {}: {:?}", delayed_name, msg);
          }

          if let Err(msg) = channel.queue_declare(
            &amqp_completed_queue,
            QueueDeclareOptions::default(),
            FieldTable::default(),
          ).wait() {
            error!("Unable to create queue {}: {:?}", amqp_completed_queue, msg);
          }

          if let Err(msg) = channel.queue_declare(
            &amqp_error_queue,
            QueueDeclareOptions::default(),
            FieldTable::default(),
          ).wait() {
            error!("Unable to create queue {}: {:?}", amqp_error_queue, msg);
          }

          let mut fields_queue = FieldTable::default();
          fields_queue.insert("x-dead-letter-exchange".into(), AMQPValue::LongString(delayed_name.into()));
          fields_queue.insert("x-dead-letter-routing-key".into(), AMQPValue::LongString(amqp_queue.clone().into()));

          channel
            .queue_declare(
              &amqp_queue,
              QueueDeclareOptions::default(),
              fields_queue,
            )
            .and_then(move |queue| {
              info!("channel {} declared queue {}", id, amqp_queue);

              channel.basic_consume(
                &queue,
                "amqp_worker",
                BasicConsumeOptions::default(),
                FieldTable::default(),
              )
            })
            .and_then(move |stream| {
              warn!("start listening stream");
              stream.for_each(move |message| {
                trace!("raw message: {:?}", message);
                let data = std::str::from_utf8(&message.data).unwrap();
                info!("got message: {}", data);

                match MessageEvent::process(message_event, data) {
                  Ok(job_result) => {
                    let msg = json!(job_result);

                    let result = ch
                      .basic_publish(
                        "", // exchange
                        &amqp_completed_queue,
                        msg.to_string().as_str().as_bytes().to_vec(),
                        BasicPublishOptions::default(),
                        BasicProperties::default(),
                      )
                      .wait();

                    if result.is_ok() {
                      if let Err(msg) = ch
                        .basic_ack(message.delivery_tag, false /*not requeue*/)
                        .wait()
                      {
                        error!("Unable to ack message {:?}", msg);
                      }
                    } else if let Err(msg) = ch
                      .basic_reject(
                        message.delivery_tag,
                        BasicRejectOptions { requeue: true }, /*requeue*/
                      )
                      .wait()
                    {
                      error!("Unable to reject message {:?}", msg);
                    }
                  }
                  Err(error) => match error {
                    MessageError::RequirementsError(msg) => {
                      debug!("{}", msg);
                      if let Err(msg) = ch
                        .basic_reject(
                          message.delivery_tag,
                          BasicRejectOptions::default(),
                        )
                        .wait()
                      {
                        error!("Unable to reject message {:?}", msg);
                      }
                    }
                    MessageError::NotImplemented() => {
                      if let Err(msg) = ch
                        .basic_reject(
                          message.delivery_tag,
                          BasicRejectOptions { requeue: true }, /*requeue*/
                        )
                        .wait()
                      {
                        error!("Unable to reject message {:?}", msg);
                      }
                    }
                    MessageError::ProcessingError(job_result) => {
                      let content = json!(JobResult {
                        job_id: job_result.job_id,
                        status: JobStatus::Error,
                        parameters: job_result.parameters,
                      });
                      if ch
                        .basic_publish(
                          "", // exchange
                          &amqp_error_queue,
                          content.to_string().as_str().as_bytes().to_vec(),
                          BasicPublishOptions::default(),
                          BasicProperties::default(),
                        )
                        .wait()
                        .is_ok()
                      {
                        if let Err(msg) = ch
                          .basic_ack(message.delivery_tag, false /*not requeue*/)
                          .wait()
                        {
                          error!("Unable to ack message {:?}", msg);
                        }
                      } else if let Err(msg) = ch
                        .basic_reject(
                          message.delivery_tag,
                          BasicRejectOptions { requeue: true }, /*requeue*/
                        )
                        .wait()
                      {
                        error!("Unable to reject message {:?}", msg);
                      }
                    }
                    MessageError::RuntimeError(msg) => {
                      let content = json!({
                        "status": "error",
                        "message": msg
                      });
                      if ch
                        .basic_publish(
                          "", // exchange
                          &amqp_error_queue,
                          content.to_string().as_str().as_bytes().to_vec(),
                          BasicPublishOptions::default(),
                          BasicProperties::default(),
                        )
                        .wait()
                        .is_ok()
                      {
                        if let Err(msg) = ch
                          .basic_ack(message.delivery_tag, false /*not requeue*/)
                          .wait()
                        {
                          error!("Unable to ack message {:?}", msg);
                        }
                      } else if let Err(msg) = ch
                        .basic_reject(
                          message.delivery_tag,
                          BasicRejectOptions { requeue: true }, /*requeue*/
                        )
                        .wait()
                      {
                        error!("Unable to reject message {:?}", msg);
                      }
                    }
                  },
                }

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
