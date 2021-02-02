pub use lapin::Channel;
pub use log::{debug, error, info, trace, warn};
pub use schemars::JsonSchema;
pub use semver::Version;
pub use std::sync::{Arc, Mutex};

pub use crate::job::{Job, JobProgression, JobResult, JobStatus};
pub use crate::message::publish_job_progression;
pub use crate::message_exchange::{
  message::{Feedback, OrderMessage, ResponseMessage},
  rabbitmq::*,
  ExternalExchange, LocalExchange, RabbitmqExchange,
};
pub use crate::parameter::{MediaSegment, MediaSegments, Parameter, ParameterValue, Requirement};
pub use crate::processor::{ProcessStatus, Processor};
pub use crate::start_worker::start_worker;
pub use crate::worker::{
  WorkerParameter, WorkerParameterType, SystemInformation, WorkerActivity, WorkerConfiguration, WorkerStatus,
};
pub use crate::{McaiChannel, MessageError, MessageEvent, Result};

#[cfg(feature = "media")]
pub use {
  crate::{
    message::media::{
      audio::AudioFormat,
      ebu_ttml_live::*,
      filters::{AudioFilter, GenericFilter, VideoFilter},
      video::{RegionOfInterest, Scaling, VideoFormat},
      StreamDescriptor
    },
    process_frame::ProcessFrame,
    process_result::ProcessResult,
  },
  stainless_ffmpeg::{format_context::FormatContext, frame::Frame},
};
