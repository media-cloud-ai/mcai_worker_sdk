extern crate futures;
#[macro_use]
extern crate log;
extern crate lapin_futures as lapin;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate tokio_core;

mod config;

use config::*;
use futures::Stream;
use futures::future::Future;

use lapin::channel::{BasicConsumeOptions, BasicProperties, BasicPublishOptions, QueueDeclareOptions};
use lapin::client::ConnectionOptions;
use lapin::types::FieldTable;
use std::net::ToSocketAddrs;
use std::{thread, time};
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;

pub trait MessageEvent {
  fn process(&self, _message: &str) -> Result<u64, MessageError>
  where
    Self: std::marker::Sized,
  {
    Err(MessageError::NotImplemented())
  }
}

#[derive(Debug)]
pub enum MessageError {
  RuntimeError(String),
  ProcessingError(u64, String),
  RequirementsError(String),
  NotImplemented(),
}

pub fn start_worker<ME: MessageEvent>(message_event: &ME) {
  let mut core = Core::new().unwrap();
  let handle = core.handle();

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
    debug!("AMQP HOSTNAME: {}", amqp_hostname);
    debug!("AMQP PORT: {}", amqp_port);
    debug!("AMQP USERNAME: {}", amqp_username);
    debug!("AMQP VHOST: {}", amqp_vhost);
    debug!("AMQP QUEUE: {}", amqp_queue);

    // create the reactor
    let address = amqp_hostname.clone() + ":" + amqp_port.as_str();
    let addr = address.to_socket_addrs().unwrap().next().unwrap();
    let channel_name = amqp_queue;

    let state = core.run(
      TcpStream::connect(&addr, &handle)
        .and_then(|stream| {
          lapin::client::Client::connect(
            stream,
            &ConnectionOptions {
              username: amqp_username,
              password: amqp_password,
              vhost: amqp_vhost,
              ..Default::default()
            },
          )
        })
        .and_then(|(client, heartbeat_future_fn)| {
          let heartbeat_client = client.clone();
          handle.spawn(heartbeat_future_fn(&heartbeat_client).map_err(|_| ()));

          client.create_channel()
        })
        .and_then(|channel| {
          let id = channel.id;
          debug!("created channel with id: {}", id);

          let ch = channel.clone();

          channel
            .queue_declare(
              &amqp_completed_queue,
              &QueueDeclareOptions::default(),
              &FieldTable::new(),
            );

          channel
            .queue_declare(
              &amqp_error_queue,
              &QueueDeclareOptions::default(),
              &FieldTable::new(),
            );

          channel
            .queue_declare(
              &channel_name,
              &QueueDeclareOptions::default(),
              &FieldTable::new(),
            )
            .and_then(move |_| {
              debug!("channel {} declared queue {}", id, channel_name);

              channel.basic_consume(
                &channel_name,
                "my_consumer",
                &BasicConsumeOptions::default(),
                &FieldTable::new(),
              )
            })
            .and_then(|stream| {
              stream.for_each(move |message| {
                let data = std::str::from_utf8(&message.data).unwrap();
                debug!("got message: {}", data);

                match MessageEvent::process(message_event, data) {
                  Ok(job_id) => {
                    let msg = json!({
                      "job_id": job_id,
                      "status": "completed"
                    });

                    if let Ok(_) = ch.basic_publish(
                      "", // exchange
                      &amqp_completed_queue,
                      msg.to_string().as_str().as_bytes(),
                      &BasicPublishOptions::default(),
                      BasicProperties::default(),
                    ).wait() {
                      ch.basic_ack(message.delivery_tag);
                    } else  {
                      let requeue = true;
                      ch.basic_reject(message.delivery_tag, requeue);
                    };
                  }
                  Err(error) => match error {
                    MessageError::RequirementsError(msg) => {
                      error!("{}", msg);
                      ch.basic_reject(message.delivery_tag, true);
                    }
                    MessageError::NotImplemented() => {
                      ch.basic_reject(message.delivery_tag, true);
                    }
                    MessageError::ProcessingError(job_id, msg) => {
                      let content = json!({
                      "status": "error",
                      "job_id": job_id,
                      "message": msg
                    });
                      if let Ok(_) = ch.basic_publish(
                        "", // exchange
                        &amqp_error_queue,
                        content.to_string().as_str().as_bytes(),
                        &BasicPublishOptions::default(),
                        BasicProperties::default(),
                      ).wait() {
                        let requeue = false;
                        ch.basic_reject(message.delivery_tag, requeue);
                      } else  {
                        let requeue = true;
                        ch.basic_reject(message.delivery_tag, requeue);
                      };
                    }
                    MessageError::RuntimeError(msg) => {
                      let content = json!({
                      "status": "error",
                      "message": msg
                    });
                      if let Ok(_) = ch.basic_publish(
                        "", // exchange
                        &amqp_error_queue,
                        content.to_string().as_str().as_bytes(),
                        &BasicPublishOptions::default(),
                        BasicProperties::default(),
                      ).wait() {
                        let requeue = false;
                        ch.basic_reject(message.delivery_tag, requeue);
                      } else  {
                        let requeue = true;
                        ch.basic_reject(message.delivery_tag, requeue);
                      };
                    }
                  },
                }
                Ok(())
              })
            })
        }),
    );

    warn!("{:?}", state);
    let sleep_duration = time::Duration::new(1, 0);
    thread::sleep(sleep_duration);
  }
}
