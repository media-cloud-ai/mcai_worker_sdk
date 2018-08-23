use config::*;
use futures::Future;
use lapin;
use lapin::channel::{BasicProperties, BasicPublishOptions, QueueDeclareOptions};
use lapin::client::ConnectionOptions;
use lapin::types::FieldTable;
use std::net::ToSocketAddrs;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;

pub fn publish(channel_name: &str, msg: String) {
  let amqp_hostname = get_amqp_hostname();
  let amqp_port = get_amqp_port();
  let amqp_username = get_amqp_username();
  let amqp_password = get_amqp_password();
  let amqp_vhost = get_amqp_vhost();

  let mut core = Core::new().unwrap();
  let handle = core.handle();
  let address = amqp_hostname.clone() + ":" + amqp_port.as_str();
  let addr = address.to_socket_addrs().unwrap().next().unwrap();

  core
    .run(
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
        .and_then(|(client, _ /* heartbeat_future_fn */)| client.create_channel())
        .and_then(|channel| {
          let id = channel.id;
          println!("created channel with id: {}", id);

          channel
            .queue_declare(
              channel_name,
              &QueueDeclareOptions::default(),
              &FieldTable::new(),
            )
            .and_then(move |_| {
              println!("Publish message on {:?}", channel_name);

              channel
                .basic_publish(
                  "", // exchange
                  channel_name,
                  msg.as_str().as_bytes(),
                  &BasicPublishOptions::default(),
                  BasicProperties::default(),
                )
                .and_then(|result| {
                  println!("{:?}", result);
                  Ok(())
                })
            })
        }),
    )
    .unwrap();
}
