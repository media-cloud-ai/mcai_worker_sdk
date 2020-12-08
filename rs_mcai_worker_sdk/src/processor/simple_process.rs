use crate::job::{Job, JobResult};
use crate::processor::Process;
use crate::{MessageEvent, Result};
use failure::_core::cell::RefCell;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::rc::Rc;

#[derive(Default)]
pub struct SimpleProcess {}

impl Process for SimpleProcess {
  fn init<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    _message_event: Rc<RefCell<ME>>,
    _job: &Job,
  ) -> Result<()> {
    Ok(())
  }

  fn start<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Rc<RefCell<ME>>,
    job: &Job,
  ) -> Result<JobResult> {
    message_event
      .borrow_mut()
      .process(None, job.get_parameters().unwrap(), JobResult::from(job))
  }

  fn stop<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    _message_event: Rc<RefCell<ME>>,
    job: &Job,
  ) -> Result<JobResult> {
    Ok(JobResult::from(job))
  }
}
