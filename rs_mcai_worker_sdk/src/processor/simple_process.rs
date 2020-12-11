use crate::{
  job::{Job, JobResult},
  processor::Process,
  McaiChannel, MessageEvent, Result,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct SimpleProcess {}

impl<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send> Process<P, ME>
  for SimpleProcess
{
  fn init(&mut self, _message_event: Arc<Mutex<ME>>, _job: &Job) -> Result<()> {
    Ok(())
  }

  fn start(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    job: &Job,
    feedback_sender: McaiChannel,
  ) -> Result<JobResult> {
    message_event.lock().unwrap().process(
      Some(feedback_sender),
      job.get_parameters().unwrap(),
      JobResult::from(job),
    )
  }

  fn stop(&mut self, _message_event: Arc<Mutex<ME>>, job: &Job) -> Result<JobResult> {
    Ok(JobResult::from(job))
  }
}
