//! Module to manage the worker (configuration and status information)

mod activity;
mod configuration;
pub mod docker;
mod status;
mod system_information;

pub use activity::WorkerActivity;
pub use configuration::WorkerConfiguration;
pub use status::WorkerStatus;
pub use system_information::SystemInformation;

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
