// use assert_matches::assert_matches;
use mcai_worker_sdk::prelude::*;
use std::sync::mpsc;

#[async_std::test]
async fn processor() -> Result<()> {
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
          if channel.lock().unwrap().is_stopped() {
            return Ok(job_result.with_status(JobStatus::Stopped));
          }
        }
      }
    }
  }

  let worker_id = "instance_id";
  let worker = Worker {};
  let worker_configuration = WorkerConfiguration::new("", &worker, worker_id).unwrap();
  let rabbitmq_exchange = RabbitmqExchange::new(&worker_configuration).await.unwrap();
  let rabbitmq_exchange = Arc::new(rabbitmq_exchange);

  let cloned_worker_configuration = worker_configuration.clone();

  let worker = Arc::new(Mutex::new(worker));

  let exchange = rabbitmq_exchange.clone();
  async_std::task::spawn(async move {
    let processor = Processor::new(exchange, cloned_worker_configuration);
    assert!(processor.run(worker).is_ok());
  });

  let (created_sender, created_receiver) = mpsc::channel::<WorkerConfiguration>();
  let (status_sender, status_receiver) = mpsc::channel::<ProcessStatus>();

  let amqp_connection = super::AmqpConnection::new().unwrap();

  amqp_connection.start_consumer(QUEUE_WORKER_CREATED, created_sender);
  amqp_connection.start_consumer(QUEUE_WORKER_STATUS, status_sender);

  let created_message = created_receiver.recv();
  assert!(created_message.is_ok());

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::Status)?;

  let status_message = status_receiver.recv();
  assert!(status_message.is_ok());

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::StopWorker)?;

  Ok(())
}
