
use crate::{
  message_exchange::{
    message::OrderMessage,
  },
  worker::WorkerConfiguration,
  McaiChannel, Result,
};
use std::sync::{Arc, Mutex};

// A trait to define common action between simple and media process
// Keep that only to facilitate testing between features
pub trait Process<P, ME> {
  fn new(
    message_event: Arc<Mutex<ME>>,
    response_sender: McaiChannel,
    worker_configuration: WorkerConfiguration,
  ) -> Self;

  fn handle(&mut self, message_event: Arc<Mutex<ME>>, order_message: OrderMessage) -> Result<()>;

  fn get_current_job_id(&self, message_event: Arc<Mutex<ME>>) -> Option<u64>;
}
