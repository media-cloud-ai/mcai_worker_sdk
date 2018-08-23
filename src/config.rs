
use std::env;

macro_rules! get_env_value {
  ($key:expr, $default:expr) => (
  {
    let mut item = $default.to_string();
    for (key, value) in env::vars() {
      match key.as_ref() {
        $key => {
          item = value;
        }
        _ => {},
      }
    }
    item
  })
}

pub fn get_amqp_hostname() -> String {
  get_env_value!("AMQP_HOSTNAME", "127.0.0.1")
}

pub fn get_amqp_port() -> String {
  get_env_value!("AMQP_PORT", "5672")
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
  get_env_value!("AMQP_QUEUE", "job_http")
}

pub fn get_amqp_completed_queue() -> String {
  get_env_value!("AMQP_COMPLETED_QUEUE", "job_http_completed")
}

pub fn get_amqp_error_queue() -> String {
  get_env_value!("AMQP_ERROR_QUEUE", "job_http_error")
}
