#[macro_use]
extern crate serde_derive;
#[cfg(feature = "media")]
extern crate stainless_ffmpeg_sys;

#[cfg(not(feature = "media"))]
mod processor {
  mod simple {
    mod local_init_job_error;
    mod local_init_start_processor;
    mod local_job_processor;
    mod local_stop_job;

    mod rabbitmq_stop_job;
  }
}

#[cfg(feature = "media")]
mod generator {
  pub mod ffmpeg;
  pub mod srt_stream;
}

#[cfg(feature = "media")]
mod media {
  use super::generator::ffmpeg;
  mod seek;
}

#[cfg(feature = "media")]
mod processor {
  use super::generator::ffmpeg;
  mod media {
    use super::*;

    mod local_complete_job;
    mod local_init_job_error;
    mod rabbitmq_stop_job;
  }
}
