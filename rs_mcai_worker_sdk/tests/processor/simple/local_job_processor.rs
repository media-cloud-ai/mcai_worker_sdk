use assert_matches::assert_matches;
use mcai_worker_sdk::prelude::*;

#[test]
fn processor() {
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
      Ok(job_result.with_message("OK"))
    }
  }

  let local_exchange = LocalExchange::new();
  let mut local_exchange = Arc::new(local_exchange);

  let worker = Worker {};
  let worker_configuration = WorkerConfiguration::new("", &worker, "instance_id").unwrap();
  let cloned_worker_configuration = worker_configuration.clone();

  let worker = Arc::new(Mutex::new(worker));

  let exchange = local_exchange.clone();
  async_std::task::spawn(async move {
    let processor = Processor::new(exchange, cloned_worker_configuration);
    assert!(processor.run(worker).is_ok());
  });
  let local_exchange = Arc::make_mut(&mut local_exchange);

  // Check if the worker is created successfully
  let response = local_exchange.next_response().unwrap();
  assert_matches!(response.unwrap(), ResponseMessage::WorkerCreated(_));

  let job = Job::new(r#"{ "job_id": 666, "parameters": [] }"#).unwrap();

  local_exchange
    .send_order(OrderMessage::Job(job.clone()))
    .unwrap();

  let response = local_exchange.next_response().unwrap();
  assert_matches!(response.unwrap(), ResponseMessage::WorkerInitialized(_));

  let response = local_exchange.next_response().unwrap();
  assert_matches!(response.unwrap(), ResponseMessage::WorkerStarted(_));

  let response = local_exchange.next_response().unwrap();
  assert_matches!(
    response.unwrap(),
    ResponseMessage::Feedback(Feedback::Progression(JobProgression{job_id: 666, progression: 0, .. }))
  );

  let response = local_exchange.next_response().unwrap();
  assert_matches!(response.unwrap(), ResponseMessage::Completed(_));

  local_exchange.send_order(OrderMessage::StopWorker).unwrap();

  let response = local_exchange.next_response().unwrap();
  println!("{:?}", response);
  assert_matches!(
    response.unwrap(),
    ResponseMessage::Feedback(Feedback::Status(ProcessStatus{
      job: None,
      worker: WorkerStatus {
        activity: WorkerActivity::Idle,
        system_info: SystemInformation {
          ..
        }
      }
    }))
  );
}
