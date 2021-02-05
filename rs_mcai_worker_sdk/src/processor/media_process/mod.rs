mod threaded_media_process;

use crate::{
  job::{JobResult, JobStatus},
  message_exchange::message::{Feedback, OrderMessage, ResponseMessage},
  processor::{Process, ProcessStatus},
  worker::{SystemInformation, WorkerConfiguration, WorkerStatus},
  McaiChannel, MessageError, MessageEvent, Result,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{
  cell::RefCell,
  ops::DerefMut,
  rc::Rc,
  sync::{
    mpsc::{channel, Sender},
    Arc, Mutex,
  },
};
use threaded_media_process::ThreadedMediaProcess;

pub struct MediaProcess {
  order_sender: Sender<OrderMessage>,
  current_job_id: Arc<Mutex<Option<u64>>>,
}

impl<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P> + Send> Process<P, ME>
  for MediaProcess
{
  fn new(
    message_event: Arc<Mutex<ME>>,
    response_sender: McaiChannel,
    worker_configuration: WorkerConfiguration,
  ) -> Self {
    let (order_sender, order_receiver) = channel();

    let status = Arc::new(Mutex::new(JobStatus::Unknown));
    let current_job_id = Arc::new(Mutex::new(None));
    let cloned_current_job_id = current_job_id.clone();

    std::thread::spawn(move || {
      let mut process_parameters: Option<Rc<RefCell<ThreadedMediaProcess>>> = None;

      let mut keep_running = true;

      // let mut received = order_receiver.recv();

      while let Ok(message) = &order_receiver.recv() {
        // Process the received order message
        let response = match message {
          OrderMessage::Job(job) => {
            log::info!("Process job: {:?}", job);
            let initialization_result =
              ThreadedMediaProcess::initialize_process(message_event.clone(), job.clone());

            if let Err(error) = initialization_result {
              (*status.lock().unwrap().deref_mut()) = JobStatus::Error;
              ResponseMessage::Error(error)
            } else {
              process_parameters = Some(Rc::new(RefCell::new(initialization_result.unwrap())));

              // TODO send worker response Initialized

              *status.lock().unwrap() = JobStatus::Running;
              *current_job_id.lock().unwrap() = Some(job.job_id);

              let response = process_parameters
                .clone()
                .unwrap()
                .borrow_mut()
                .start_process(
                  message_event.clone(),
                  &order_receiver,
                  response_sender.clone(),
                  worker_configuration.clone(),
                );

              *status.lock().unwrap() = if matches!(response, ResponseMessage::Error(_)) {
                JobStatus::Error
              } else {
                JobStatus::Completed
              };

              *current_job_id.lock().unwrap() = None;

              response
            }
          }
          OrderMessage::InitProcess(job) => {
            let initialization_result =
              ThreadedMediaProcess::initialize_process(message_event.clone(), job.clone());

            if let Err(error) = initialization_result {
              *status.lock().unwrap() = JobStatus::Error;
              ResponseMessage::Error(error)
            } else {
              *status.lock().unwrap() = JobStatus::Initialized;
              *current_job_id.lock().unwrap() = Some(job.job_id);

              process_parameters = Some(Rc::new(RefCell::new(initialization_result.unwrap())));

              ResponseMessage::WorkerInitialized(
                JobResult::new(job.job_id).with_status(JobStatus::Initialized),
              )
            }
          }
          OrderMessage::StartProcess(job) => {
            *status.lock().unwrap() = JobStatus::Running;

            let response = if let Some(media_process_parameters) = &process_parameters {
              media_process_parameters.borrow_mut().start_process(
                message_event.clone(),
                &order_receiver,
                response_sender.clone(),
                worker_configuration.clone(),
              )
            } else {
              ResponseMessage::Error(MessageError::ProcessingError(
                JobResult::new(job.job_id)
                  .with_status(JobStatus::Error)
                  .with_message("Process cannot be started, it must be initialized before!"),
              ))
            };

            log::info!("Finished response: {:?}", response);

            *status.lock().unwrap() = if matches!(response, ResponseMessage::Error(_)) {
              JobStatus::Error
            } else {
              JobStatus::Running
            };

            response
          }
          OrderMessage::StopProcess(job) => ResponseMessage::Error(MessageError::ProcessingError(
            JobResult::new(job.job_id)
              .with_status(JobStatus::Error)
              .with_message("Cannot stop a non-running job."),
          )),
          OrderMessage::Status => Self::get_status_feedback(
            status.lock().unwrap().clone(),
            process_parameters.clone(),
            worker_configuration.clone(),
          ),
          OrderMessage::StopWorker => {
            keep_running = false;
            Self::get_status_feedback(
              status.lock().unwrap().clone(),
              process_parameters.clone(),
              worker_configuration.clone(),
            )
          }
        };

        match response {
          ResponseMessage::Completed(_)
          | ResponseMessage::Error(_)
          | ResponseMessage::JobStopped(_) => {
            *current_job_id.lock().unwrap() = None;
          }
          _ => {}
        }

        // Send the action response
        log::trace!("Send the action response message");
        response_sender
          .lock()
          .unwrap()
          .send_response(response)
          .unwrap();

        // If the process is stopped, stop looping
        if !keep_running {
          break;
        }
      }
    });

    MediaProcess {
      order_sender,
      current_job_id: cloned_current_job_id,
    }
  }

  fn handle(&mut self, _message_event: Arc<Mutex<ME>>, order_message: OrderMessage) -> Result<()> {
    if let Err(error) = self.order_sender.send(order_message) {
      return Err(MessageError::RuntimeError(error.to_string())); // TODO use ProcessError
    }
    Ok(())
  }

  fn get_current_job_id(&self, _message_event: Arc<Mutex<ME>>) -> Option<u64> {
    *self.current_job_id.lock().unwrap()
  }
}

impl MediaProcess {
  fn get_status_feedback(
    status: JobStatus,
    process_parameters: Option<Rc<RefCell<ThreadedMediaProcess>>>,
    worker_configuration: WorkerConfiguration,
  ) -> ResponseMessage {
    let job_result = process_parameters
      .map(|param| JobResult::new(param.borrow().job.job_id).with_status(status.clone()));

    get_status_feedback(status, job_result, worker_configuration)
  }
}

fn get_status_feedback(
  status: JobStatus,
  job_result: Option<JobResult>,
  worker_configuration: WorkerConfiguration,
) -> ResponseMessage {
  let activity = status.into();
  let system_info = SystemInformation::new(&worker_configuration);

  ResponseMessage::Feedback(Feedback::Status(ProcessStatus::new(
    WorkerStatus::new(activity, system_info),
    job_result,
  )))
}
