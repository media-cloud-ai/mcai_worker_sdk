use crate::{
  job::{Job, JobProgression, JobResult, JobStatus},
  message_exchange::message::{Feedback, OrderMessage, ResponseMessage},
  processor::{Process, ProcessStatus},
  worker::{SystemInformation, WorkerActivity, WorkerConfiguration, WorkerStatus},
  McaiChannel, MessageEvent, Result,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

pub struct SimpleProcess {
  response_sender: McaiChannel,
  status: Arc<Mutex<JobStatus>>,
  worker_configuration: WorkerConfiguration,
  current_job_id: Arc<Mutex<Option<u64>>>,
}

impl<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send> Process<P, ME>
  for SimpleProcess
{
  fn new(
    _message_event: Arc<Mutex<ME>>,
    response_sender: McaiChannel,
    worker_configuration: WorkerConfiguration,
  ) -> Self {
    SimpleProcess {
      response_sender,
      status: Arc::new(Mutex::new(JobStatus::Unknown)),
      worker_configuration,
      current_job_id: Arc::new(Mutex::new(None)),
    }
  }

  fn handle(&mut self, message_event: Arc<Mutex<ME>>, order_message: OrderMessage) -> Result<()> {
    let response =
      match order_message {
        OrderMessage::Job(job) => {
          *self.status.lock().unwrap() = JobStatus::Initialized;
          *self.current_job_id.lock().unwrap() = Some(job.job_id);

          self.response_sender.lock().unwrap().send_response(
            ResponseMessage::WorkerInitialized(
              JobResult::new(job.job_id).with_status(JobStatus::Initialized),
            ),
          )?;

          *self.status.lock().unwrap() = JobStatus::Running;
          self.execute(message_event, &job);
          None
        }
        OrderMessage::InitProcess(job) => {
          *self.status.lock().unwrap() = JobStatus::Initialized;
          *self.current_job_id.lock().unwrap() = Some(job.job_id);

          Some(ResponseMessage::WorkerInitialized(
            JobResult::new(job.job_id).with_status(JobStatus::Initialized),
          ))
        }
        OrderMessage::StartProcess(job) => {
          *self.status.lock().unwrap() = JobStatus::Running;
          self.execute(message_event, &job);

          None
        }
        // Nothing to do here to stop the current job
        OrderMessage::StopProcess(_job) => None,
        OrderMessage::Status | OrderMessage::StopWorker => {
          let status = self.status.lock().unwrap().clone();
          let current_job_result = self
            .current_job_id
            .lock()
            .unwrap()
            .map(|job_id| JobResult::new(job_id).with_status(status));

          Some(ResponseMessage::Feedback(Feedback::Status(
            ProcessStatus::new(self.get_worker_status(), current_job_result),
          )))
        }
      };

    if let Some(response) = response {
      self
        .response_sender
        .lock()
        .unwrap()
        .send_response(response)?;
    }
    Ok(())
  }

  fn get_current_job_id(&self, _message_event: Arc<Mutex<ME>>) -> Option<u64> {
    *self.current_job_id.lock().unwrap()
  }
}

impl SimpleProcess {
  fn get_worker_status(&self) -> WorkerStatus {
    let activity = self.get_worker_activity();
    let system_info = SystemInformation::new(&self.worker_configuration.clone());
    WorkerStatus::new(activity, system_info)
  }

  fn get_worker_activity(&self) -> WorkerActivity {
    self.status.lock().unwrap().clone().into()
  }

  fn execute<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send>(
    &mut self,
    message_event: Arc<Mutex<ME>>,
    job: &Job,
  ) {
    let response_sender = self.response_sender.clone();
    let message_event = message_event;
    let status = self.status.clone();
    let current_job_id = self.current_job_id.clone();
    let job = job.clone();

    spawn(move || {
      let job_id = job.job_id;

      let worker_started = ResponseMessage::WorkerStarted(
        JobResult::from(job.clone()).with_status(JobStatus::Running),
      );

      response_sender
        .lock()
        .unwrap()
        .send_response(worker_started)
        .unwrap();

      // start publishing progression
      let feedback =
        ResponseMessage::Feedback(Feedback::Progression(JobProgression::new(job_id, 0)));

      response_sender
        .lock()
        .unwrap()
        .send_response(feedback)
        .unwrap();

      let response = match job.get_parameters() {
        Ok(parameters) => message_event.lock().unwrap().process(
          Some(response_sender.clone()),
          parameters,
          JobResult::from(job),
        ),
        Err(error) => Err(error),
      };

      let response = if response_sender.lock().unwrap().is_stopped() {
        response
          .map(ResponseMessage::JobStopped)
          .unwrap_or_else(ResponseMessage::Error)
      } else {
        response
          .map(ResponseMessage::Completed)
          .unwrap_or_else(ResponseMessage::Error)
      };

      *status.lock().unwrap() = response.clone().into();

      *current_job_id.lock().unwrap() = None;

      response_sender
        .lock()
        .unwrap()
        .send_response(response)
        .unwrap();
    });
  }
}

impl Drop for SimpleProcess {
  fn drop(&mut self) {
    log::info!("Simple process dropped with status: {:?}", self.status);
  }
}
