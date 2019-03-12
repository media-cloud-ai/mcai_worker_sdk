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

use config::*;
use failure::Error;
use futures::future::Future;
use futures::Stream;
use lapin::channel::{
  BasicConsumeOptions, BasicProperties, BasicPublishOptions, BasicQosOptions, QueueDeclareOptions,
};
use lapin::client::ConnectionOptions;
use lapin::types::FieldTable;
use std::net::ToSocketAddrs;
use std::{thread, time};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;

pub trait MessageEvent {
  fn process(&self, _message: &str) -> Result<u64, MessageError>
  where
    Self: std::marker::Sized,
  {
    Err(MessageError::NotImplemented())
  }
}

#[derive(Debug, PartialEq)]
pub enum MessageError {
  RuntimeError(String),
  ProcessingError(u64, String),
  RequirementsError(String),
  NotImplemented(),
}

pub fn start_worker<ME: MessageEvent>(message_event: &'static ME)
where
  ME: std::marker::Sync,
{
  loop {
    let amqp_hostname = get_amqp_hostname();
    let amqp_port = get_amqp_port();
    let amqp_username = get_amqp_username();
    let amqp_password = get_amqp_password();
    let amqp_vhost = get_amqp_vhost();
    let amqp_queue = get_amqp_queue();
    let amqp_completed_queue = get_amqp_completed_queue();
    let amqp_error_queue = get_amqp_error_queue();

    info!("Start connection with configuration:");
    info!("AMQP HOSTNAME: {}", amqp_hostname);
    info!("AMQP PORT: {}", amqp_port);
    info!("AMQP USERNAME: {}", amqp_username);
    info!("AMQP VHOST: {}", amqp_vhost);
    info!("AMQP QUEUE: {}", amqp_queue);

    let address = amqp_hostname.clone() + ":" + amqp_port.as_str();
    let addr = address.to_socket_addrs().unwrap().next().unwrap();

    let state = Runtime::new().unwrap().block_on_all(
      TcpStream::connect(&addr)
        .map_err(Error::from)
        .and_then(|connection| {
          lapin::client::Client::connect(
            connection,
            ConnectionOptions {
              username: amqp_username,
              password: amqp_password,
              vhost: amqp_vhost,
              ..Default::default()
            },
          )
          .map_err(Error::from)
        })
        .and_then(|(client, heartbeat)| {
          tokio::spawn(heartbeat.map_err(|e| eprintln!("heartbeat error: {}", e)));
          client.create_channel().map_err(Error::from)
        })
        .and_then(move |channel| {
          let id = channel.id;
          debug!("created channel with id: {}", id);

          if let Err(msg) = channel
            .basic_qos(BasicQosOptions {
              prefetch_size: 0,
              prefetch_count: 1,
              global: true,
            })
            .wait()
          {
            error!("Unable to set QoS on channels: {:?}", msg);
          }

          let ch = channel.clone();

          channel.queue_declare(
            &amqp_completed_queue,
            QueueDeclareOptions::default(),
            FieldTable::new(),
          );

          channel.queue_declare(
            &amqp_error_queue,
            QueueDeclareOptions::default(),
            FieldTable::new(),
          );

          channel
            .queue_declare(
              &amqp_queue,
              QueueDeclareOptions::default(),
              FieldTable::new(),
            )
            .and_then(move |queue| {
              info!("channel {} declared queue {}", id, amqp_queue);

              channel.basic_consume(
                &queue,
                "amqp_worker",
                BasicConsumeOptions::default(),
                FieldTable::new(),
              )
            })
            .and_then(move |stream| {
              warn!("start listening stream");
              stream.for_each(move |message| {
                info!("raw message: {:?}", message);
                let data = std::str::from_utf8(&message.data).unwrap();
                info!("got message: {}", data);

                match MessageEvent::process(message_event, data) {
                  Ok(job_id) => {
                    let msg = json!({
                      "job_id": job_id,
                      "status": "completed"
                    });

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
                    } else {
                      if let Err(msg) = ch
                        .basic_reject(message.delivery_tag, true /*requeue*/)
                        .wait()
                      {
                        error!("Unable to reject message {:?}", msg);
                      }
                    }
                  }
                  Err(error) => match error {
                    MessageError::RequirementsError(msg) => {
                      error!("{}", msg);
                      if let Err(msg) = ch
                        .basic_reject(message.delivery_tag, true /*requeue*/)
                        .wait()
                      {
                        error!("Unable to reject message {:?}", msg);
                      }
                    }
                    MessageError::NotImplemented() => {
                      if let Err(msg) = ch
                        .basic_reject(message.delivery_tag, true /*requeue*/)
                        .wait()
                      {
                        error!("Unable to reject message {:?}", msg);
                      }
                    }
                    MessageError::ProcessingError(job_id, msg) => {
                      let content = json!({
                        "status": "error",
                        "job_id": job_id,
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
                      } else {
                        if let Err(msg) = ch
                          .basic_reject(message.delivery_tag, true /*requeue*/)
                          .wait()
                        {
                          error!("Unable to reject message {:?}", msg);
                        }
                      };
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
                      } else {
                        if let Err(msg) = ch
                          .basic_reject(message.delivery_tag, true /*requeue*/)
                          .wait()
                        {
                          error!("Unable to reject message {:?}", msg);
                        }
                      };
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
