use mcai_worker_sdk::client::AmqpConnection;
use mcai_worker_sdk::prelude::*;
use std::sync::mpsc;

#[async_std::test]
async fn rabbitmq_stop_job() -> Result<()> {
  env_logger::init();

  struct Worker {}

  #[derive(Clone, Debug, Deserialize, JsonSchema)]
  pub struct WorkerParameters {}

  impl MessageEvent<WorkerParameters> for Worker {
    fn get_name(&self) -> String {
      "Test Worker".to_string()
    }

    fn get_short_description(&self) -> String {
      "The Worker defined in unit tests".to_string()
    }

    fn get_description(&self) -> String {
      "Mock a Worker to realise tests around SDK".to_string()
    }

    fn get_version(&self) -> semver::Version {
      semver::Version::parse("1.2.3").unwrap()
    }

    fn init(&mut self) -> Result<()> {
      log::info!("Initialize processor test worker!");
      Ok(())
    }

    fn process(
      &self,
      channel: Option<McaiChannel>,
      _parameters: WorkerParameters,
      job_result: JobResult,
    ) -> Result<JobResult>
    where
      Self: std::marker::Sized,
    {
      assert!(channel.is_some());
      loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if let Some(channel) = &channel {
          let is_stopped = channel.lock().unwrap().is_stopped();
          log::warn!("Worker is stopped: {}", is_stopped);
          if is_stopped {
            return Ok(job_result.with_status(JobStatus::Stopped));
          }
        }
      }
    }
  }

  let instance_id = "9876543210";
  let worker = Worker {};
  let worker_configuration = WorkerConfiguration::new("", &worker, instance_id).unwrap();
  let rabbitmq_exchange = RabbitmqExchange::new(&worker_configuration).await;

  if let Err(MessageError::Amqp(lapin::Error::IOError(error))) = rabbitmq_exchange {
    eprintln!(
      "Connection to RabbitMQ failure: {}. Skip test.",
      error.to_string()
    );
    return Ok(());
  }

  let rabbitmq_exchange = Arc::new(rabbitmq_exchange.unwrap());

  let cloned_worker_configuration = worker_configuration.clone();

  let worker = Arc::new(Mutex::new(worker));

  let exchange = rabbitmq_exchange.clone();
  async_std::task::spawn(async move {
    let processor = Processor::new(exchange, cloned_worker_configuration);
    assert!(processor.run(worker).is_ok());
  });

  let (created_sender, created_receiver) = mpsc::channel::<WorkerConfiguration>();
  let (status_sender, status_receiver) = mpsc::channel::<ProcessStatus>();
  let (initialized_sender, initialized_receiver) = mpsc::channel::<JobResult>();
  let (started_sender, started_receiver) = mpsc::channel::<JobResult>();

  let (progression_sender, progression_receiver) = mpsc::channel::<JobProgression>();
  let (stopped_sender, stopped_receiver) = mpsc::channel::<JobResult>();

  let amqp_connection = AmqpConnection::new().unwrap();

  amqp_connection.start_consumer(QUEUE_WORKER_CREATED, created_sender);
  amqp_connection.start_consumer(QUEUE_WORKER_STATUS, status_sender);
  amqp_connection.start_consumer(QUEUE_WORKER_INITIALIZED, initialized_sender);
  amqp_connection.start_consumer(QUEUE_WORKER_STARTED, started_sender);

  amqp_connection.start_consumer(QUEUE_JOB_PROGRESSION, progression_sender);
  amqp_connection.start_consumer(QUEUE_JOB_STOPPED, stopped_sender);

  assert!(created_receiver.recv().is_ok());

  amqp_connection.send_order(vec![instance_id], &OrderMessage::Status)?;
  assert!(status_receiver.recv().is_ok());

  let job = Job::new(r#"{ "job_id": 666, "parameters": [] }"#).unwrap();

  amqp_connection.send_order(vec![instance_id], &OrderMessage::InitProcess(job.clone()))?;
  assert!(initialized_receiver.recv().is_ok());

  amqp_connection.send_order(vec![instance_id], &OrderMessage::StartProcess(job.clone()))?;

  assert!(started_receiver.recv().is_ok());
  assert!(progression_receiver.recv().is_ok());

  amqp_connection.send_order(vec![instance_id], &OrderMessage::StopProcess(job))?;
  assert!(stopped_receiver.recv().is_ok());

  Ok(())
}
