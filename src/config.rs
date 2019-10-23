use std::env;

macro_rules! get_env_value {
  ($key:expr, $default:expr) => {
    match env::var($key) {
      Ok(value) => value,
      _ => $default.to_string(),
    }
  };
}

pub fn get_amqp_tls() -> bool {
  let value = get_env_value!("AMQP_TLS", "true");
  match value.as_str() {
    "true" | "1" | "True" | "TRUE" => true,
    _ => false,
  }
}

pub fn get_amqp_hostname() -> String {
  get_env_value!("AMQP_HOSTNAME", "127.0.0.1")
}

pub fn get_amqp_port() -> u16 {
  let value = get_env_value!("AMQP_PORT", "5672");
  match value.parse::<u16>() {
    Ok(value) => value,
    _ => 5672,
  }
}

pub fn get_amqp_username() -> String {
  get_env_value!("AMQP_USERNAME", "guest")
}

pub fn get_amqp_password() -> String {
  get_env_value!("AMQP_PASSWORD", "guest")
}

pub fn get_amqp_vhost() -> String {
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
