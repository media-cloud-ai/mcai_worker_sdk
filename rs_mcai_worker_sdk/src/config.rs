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
  matches!(value.as_str(), "true" | "1" | "True" | "TRUE")
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
  get_env_value!("AMQP_VHOST", get_env_value!("AMQP_VIRTUAL_HOST", "/"))
}

pub fn get_amqp_queue() -> String {
  get_env_value!("AMQP_QUEUE", "job_undefined")
}

pub fn get_store_hostname(store_code: &str) -> String {
  get_env_value!(
    &format!("{}_HOSTNAME", store_code),
    "http://127.0.0.1:4000/api"
  )
}

pub fn get_store_username(store_code: &str) -> String {
  get_env_value!(&format!("{}_USERNAME", store_code), "")
}

pub fn get_store_password(store_code: &str) -> String {
  get_env_value!(&format!("{}_PASSWORD", store_code), "")
}

pub fn get_amqp_uri() -> AMQPUri {
  let amqp_tls = get_amqp_tls();
  let amqp_hostname = get_amqp_hostname();
  let amqp_port = get_amqp_port();
  let amqp_username = get_amqp_username();
  let amqp_password = get_amqp_password();
  let amqp_vhost = get_amqp_vhost();
  let amqp_queue = get_amqp_queue();

  log::info!("Start connection with configuration:");
  log::info!("AMQP TLS: {}", amqp_tls);
  log::info!("AMQP HOSTNAME: {}", amqp_hostname);
  log::info!("AMQP PORT: {}", amqp_port);
  log::info!("AMQP USERNAME: {}", amqp_username);
  log::info!("AMQP VIRTUAL HOST: {}", amqp_vhost);
  log::info!("AMQP QUEUE: {}", amqp_queue);

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

pub fn get_source_orders() -> Option<Vec<String>> {
  env::var("SOURCE_ORDERS")
    .map(|source_orders| {
      Some(
        source_orders
          .split(':')
          .map(|path| path.to_string())
          .collect(),
      )
    })
    .unwrap_or(None)
}

#[test]
fn configuration() {
  assert!(get_amqp_hostname() == *"127.0.0.1");
  assert!(get_amqp_port() == 5672);
  assert!(get_amqp_username() == *"guest");
  assert!(get_amqp_password() == *"guest");
  assert!(get_amqp_queue() == *"job_undefined");
  assert!(get_store_hostname("BACKEND") == *"http://127.0.0.1:4000/api");
  assert!(get_store_username("BACKEND") == *"");
  assert!(get_store_password("BACKEND") == *"");

  env::set_var("AMQP_TLS", "1");
  assert!(get_amqp_tls());
  env::set_var("AMQP_TLS", "False");
  assert!(!get_amqp_tls());
  env::set_var("AMQP_VHOST", "/");
  assert!(get_amqp_vhost() == *"/");
  env::set_var("AMQP_PORT", "BAD_VALUE");
  assert!(get_amqp_port() == 5672);
}
