use amq_protocol_uri::{AMQPAuthority, AMQPScheme, AMQPUri, AMQPUserInfo};
use std::env;

macro_rules! get_env_value {
  ($key:expr, $default:expr) => {
    match env::var($key) {
      Ok(value) => value,
      _ => $default.to_string(),
    }
  };
}

fn get_amqp_tls() -> bool {
  let value = get_env_value!("AMQP_TLS", "true");
  match value.as_str() {
    "true" | "1" | "True" | "TRUE" => true,
    _ => false,
  }
}

fn get_amqp_hostname() -> String {
  get_env_value!("AMQP_HOSTNAME", "127.0.0.1")
}

fn get_amqp_port() -> u16 {
  let value = get_env_value!("AMQP_PORT", "5672");
  match value.parse::<u16>() {
    Ok(value) => value,
    _ => 5672,
  }
}

fn get_amqp_username() -> String {
  get_env_value!("AMQP_USERNAME", "guest")
}

fn get_amqp_password() -> String {
  get_env_value!("AMQP_PASSWORD", "guest")
}

fn get_amqp_vhost() -> String {
  get_env_value!("AMQP_VHOST", "/")
}

pub fn get_amqp_queue() -> String {
  get_env_value!("AMQP_QUEUE", "job_undefined")
}

pub fn get_backend_hostname() -> String {
  get_env_value!("BACKEND_HOSTNAME", "http://127.0.0.1:4000/api")
}

pub fn get_backend_username() -> String {
  get_env_value!("BACKEND_USERNAME", "")
}

pub fn get_backend_password() -> String {
  get_env_value!("BACKEND_PASSWORD", "")
}

pub fn get_amqp_uri() -> AMQPUri {
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

  AMQPUri {
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
  }
}

#[test]
fn configuration() {
  assert!(get_amqp_tls() == true);
  assert!(get_amqp_hostname() == "127.0.0.1".to_string());
  assert!(get_amqp_port() == 5672);
  assert!(get_amqp_username() == "guest".to_string());
  assert!(get_amqp_password() == "guest".to_string());
  assert!(get_amqp_vhost() == "/".to_string());
  assert!(get_amqp_queue() == "job_undefined".to_string());
  assert!(get_backend_hostname() == "http://127.0.0.1:4000/api".to_string());
  assert!(get_backend_username() == "".to_string());
  assert!(get_backend_password() == "".to_string());

  env::set_var("AMQP_TLS", "False");
  assert!(get_amqp_tls() == false);
  env::set_var("AMQP_PORT", "BAD_VALUE");
  assert!(get_amqp_port() == 5672);
}
