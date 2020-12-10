use std::sync::Arc;

use async_std::{
  channel::Receiver,
  task::{spawn, JoinHandle},
};
use lapin::Channel;

use crate::message_exchange::Feedback::Progression;
use crate::{
  message_exchange::{rabbitmq::publish::job_progression::job_progression, Feedback},
  Result,
};

pub struct FeedbackPublisher {
  handle: Option<JoinHandle<()>>,
}

impl FeedbackPublisher {
  pub fn new(channel: &Channel, receiver: Receiver<Feedback>) -> Result<FeedbackPublisher> {
    let channel = Arc::new(channel.clone());

    let handle = Some(spawn(async move {
      while let Ok(feedback) = receiver.recv().await {
        match feedback {
          Progression(job_prog) => job_progression(Some(channel.clone()), job_prog).unwrap(),
        }
      }
    }));

    Ok(FeedbackPublisher { handle })
  }
}

impl Drop for FeedbackPublisher {
  fn drop(&mut self) {
    self.handle.take().map(JoinHandle::cancel);
  }
}
