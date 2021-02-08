use assert_matches::assert_matches;
use mcai_worker_sdk::prelude::*;

#[test]
fn processor_initialization_error() {
  struct Worker {}

  #[derive(Clone, Debug, Deserialize, JsonSchema)]
  pub struct WorkerParameters {
    credential: String,
  }

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
      _channel: Option<McaiChannel>,
      _parameters: WorkerParameters,
      _job_result: JobResult,
    ) -> Result<JobResult>
      where
        Self: std::marker::Sized,
    {
      unimplemented!();
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

  let message = r#"{ "job_id": 666, "parameters": [
      { "id": "credential", "store": "backend", "type": "string", "value": "credential_key" }
    ] }"#;

  let job = Job::new(message).unwrap();
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

  let expected_error_message =
    "\"error sending request for url (http://127.0.0.1:4000/api/sessions): \
    error trying to connect: tcp connect error: Connection refused (os error 111)\""
      .to_string();
  let expected_error = MessageError::ParameterValueError(expected_error_message);
  assert_eq!(response.unwrap(), ResponseMessage::Error(expected_error));
}
