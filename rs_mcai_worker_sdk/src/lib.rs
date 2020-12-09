//! # MCAI Worker SDK
//!
//! This library is an SDK to communicate via message broker with [StepFlow](https://hexdocs.pm/step_flow/readme.html).
//! It's used for every worker as an abstraction.
//! It manage itself requirements, message parsing, direct messaging.
//!
//! ## Worker implementation
//!
//! 1. Create a Rust project
//! 2. Add MCAI Worker SDK as a dependency in Cargo.toml: `mcai_worker_sdk = "^1.0"`
//! 1. Update the main file with the example provided here to implement [MessageEvent](trait.MessageEvent.html) trait,
//! and call the [`start_worker`](fn.start_worker.html) to start the worker itself.
//!
//! ```rust
//! use mcai_worker_sdk::{
//!   MessageEvent,
//!   Version,
//!   worker::Parameter,
//! };
//! use serde_derive::Deserialize;
//! use schemars::JsonSchema;
//!
//! #[derive(Debug)]
//! struct WorkerNameEvent {}
//!
//! #[derive(Debug, Deserialize, JsonSchema)]
//! struct WorkerParameters {}
//!
//! impl MessageEvent<WorkerParameters> for WorkerNameEvent {
//!   fn get_name(&self) -> String {"sample_worker".to_string()}
//!   fn get_short_description(&self) -> String {"Short description".to_string()}
//!   fn get_description(&self) -> String {"Long description".to_string()}
//!   fn get_version(&self) -> Version { Version::new(0, 0, 1) }
//! }
//! static WORKER_NAME_EVENT: WorkerNameEvent = WorkerNameEvent {};
//!
//! // uncomment it to start the worker
//! // fn main() {
//! //   mcai_worker_sdk::start_worker(&WORKER_NAME_EVENT);
//! // }
//! ```
//!
//! ## Runtime configuration
//!
//! ### AMQP connection
//!
//! |    Variable     | Description |
//! |-----------------|-------------|
//! | `AMQP_HOSTNAME` | IP or host of AMQP server (default: `localhost`) |
//! | `AMQP_PORT`     | AMQP server port (default: `5672`) |
//! | `AMQP_TLS`      | enable secure connection using AMQPS (default: `false`, enable with `true` or `1` or `TRUE` or `True`) |
//! | `AMQP_USERNAME` | Username used to connect to AMQP server (default: `guest`) |
//! | `AMQP_PASSWORD` | Password used to connect to AMQP server (default: `guest`) |
//! | `AMQP_VHOST`    | AMQP virtual host (default: `/`) |
//! | `AMQP_QUEUE`    | AMQP queue name used to receive job orders (default: `job_undefined`) |
//!
//! ### Vault connection
//!
//! |    Variable        | Description |
//! |--------------------|-------------|
//! | `BACKEND_HOSTNAME` | URL used to connect to backend server (default: `http://127.0.0.1:4000/api`) |
//! | `BACKEND_USERNAME` | Username used to connect to backend server |
//! | `BACKEND_PASSWORD` | Password used to connect to backend server |
//!
//! ## Start worker locally
//!
//! MCAI Worker SDK can be launched locally - without RabbitMQ.
//! It can process some message for different purpose (functional tests, message order examples, etc.).
//!
//! To start worker in this mode, setup the environment variable `SOURCE_ORDERS` with path(s) to json orders.
//! It can take multiple orders, joined with `:` on unix platform, `;` on windows os.
//!
//! ### Examples:
//!
//! ```bash
//! RUST_LOG=info SOURCE_ORDERS=./examples/success_order.json:./examples/error_order.json cargo run --example worker
//! ```

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[cfg(feature = "media")]
#[macro_use]
extern crate yaserde_derive;

mod channels;
mod config;
mod error;
pub mod job;
pub mod message;
pub mod message_event;
pub mod parameter;
#[cfg(feature = "media")]
mod process_result;
#[cfg(feature = "media")]
mod process_frame;
mod start_worker;
pub mod worker;

pub mod message_exchange;
pub mod processor;

/// Re-export from lapin Channel
pub use lapin::Channel;
pub use log::{debug, error, info, trace, warn};
pub use schemars::JsonSchema;
/// Re-export from semver:
pub use semver::Version;

pub use error::{MessageError, Result};
#[cfg(feature = "media")]
pub use message::media::{
  audio::AudioFormat,
  ebu_ttml_live::{
    Body, Div, EbuTtmlLive, Frames, Head, Paragraph, Span, Styling, TimeExpression, TimeUnit, Title,
  },
  filters::{AudioFilter, GenericFilter, VideoFilter},
  video::{RegionOfInterest, Scaling, VideoFormat},
  StreamDescriptor,
};
pub use message::publish_job_progression;
pub use message_event::MessageEvent;
pub use parameter::container::ParametersContainer;
pub use parameter::{Parameter, ParameterValue, Requirement};
#[cfg(feature = "media")]
pub use process_result::ProcessResult;
#[cfg(feature = "media")]
pub use process_frame::ProcessFrame;
use processor::Processor;
#[cfg(feature = "media")]
pub use stainless_ffmpeg::{format_context::FormatContext, frame::Frame};
pub use start_worker::start_worker;

use job::JobResult;
use std::sync::Arc;

/// Exposed Channel type
pub type McaiChannel = Arc<Channel>;

#[test]
fn empty_message_event_impl() {
  #[derive(Debug)]
  struct CustomEvent {}

  #[derive(JsonSchema, Deserialize)]
  struct CustomParameters {}

  impl MessageEvent<CustomParameters> for CustomEvent {
    fn get_name(&self) -> String {
      "custom".to_string()
    }
    fn get_short_description(&self) -> String {
      "short description".to_string()
    }
    fn get_description(&self) -> String {
      "long description".to_string()
    }
    fn get_version(&self) -> semver::Version {
      semver::Version::new(1, 2, 3)
    }
  }

  let custom_event = CustomEvent {};
  let parameters = CustomParameters {};

  let job = job::Job {
    job_id: 1234,
    parameters: vec![],
  };

  let job_result = job::JobResult::new(job.job_id);

  let result = custom_event.process(None, parameters, job_result);
  assert!(result == Err(MessageError::NotImplemented()));
}
