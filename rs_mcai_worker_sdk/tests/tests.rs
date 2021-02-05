#[macro_use]
extern crate serde_derive;
#[cfg(feature = "media")]
extern crate stainless_ffmpeg_sys;

mod amqp {
  pub mod connection;
}

#[cfg(not(feature = "media"))]
mod processor {
  use super::amqp::connection::*;
  mod rabbitmq_simple_stop_job;

  mod simple_job_processor;
  mod simple_processor;
  mod simple_stop_job;
}

#[cfg(feature = "media")]
mod generator {
  pub mod ffmpeg;
}

#[cfg(feature = "media")]
mod media {
  use super::generator::ffmpeg;
  mod seek;
}

#[cfg(feature = "media")]
mod processor {
  use super::amqp::connection::*;
  use super::generator::ffmpeg;
  mod media {
    use super::*;

    mod local_complete_job;
    mod rabbitmq_stop_job;
  }
}
