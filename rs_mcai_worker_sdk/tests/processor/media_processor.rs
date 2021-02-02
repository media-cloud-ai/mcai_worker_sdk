use assert_matches::assert_matches;
use mcai_worker_sdk::{
  job::{Job, JobResult},
  message_exchange::{ExternalExchange, Feedback, LocalExchange, OrderMessage, ResponseMessage},
  processor::Processor,
  worker::WorkerConfiguration,
  JsonSchema, MessageEvent, ProcessFrame, ProcessResult, Result,
};
use std::sync::{Arc, Mutex};

#[test]
fn processor() {
  let file_path = "./test_media_processor.mxf";
  let nb_frames = 50;
  super::ffmpeg::create_xdcam_sample_file(file_path, nb_frames).unwrap();

  struct Worker {}

  #[derive(Clone, Debug, Deserialize, JsonSchema)]
  pub struct WorkerParameters {
    source_path: String,
    destination_path: String,
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

    fn process_frame(
      &mut self,
      _job_result: JobResult,
      _stream_index: usize,
      _frame: ProcessFrame,
    ) -> Result<ProcessResult> {
      assert!(false);
      log::info!("Process frame");
      Ok(ProcessResult::new_json(""))
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

  let job = Job::new(
    r#"{
    "job_id": 999,
    "parameters": [
      {
        "id": "source_path",
        "type": "string",
        "value": "./test_media_processor.mxf"
      },
      {
        "id": "destination_path",
        "type": "string",
        "value": "/test_media_processor.mp4"
      }
    ]
  }"#,
  )
  .unwrap();

  local_exchange
    .send_order(OrderMessage::InitProcess(job.clone()))
    .unwrap();

  let response = local_exchange.next_response().unwrap();
  assert_matches!(response.unwrap(), ResponseMessage::WorkerInitialized(_));

  local_exchange
    .send_order(OrderMessage::StartProcess(job.clone()))
    .unwrap();

  let response = local_exchange.next_response().unwrap();
  assert_matches!(
    response.unwrap(),
    ResponseMessage::WorkerStarted(JobResult { .. })
  );

  let response = local_exchange.next_response().unwrap();
  assert_matches!(
    response.unwrap(),
    ResponseMessage::Feedback(Feedback::Progression { .. })
  );

  local_exchange.send_order(OrderMessage::Status).unwrap();

  let response = local_exchange.next_response().unwrap();
  assert_matches!(
    response.unwrap(),
    ResponseMessage::Feedback(Feedback::Status { .. })
  );

  local_exchange
    .send_order(OrderMessage::StopProcess(job.clone()))
    .unwrap();

  let response = local_exchange.next_response().unwrap();
  assert_matches!(response.unwrap(), ResponseMessage::Completed(_));

  local_exchange.send_order(OrderMessage::StopWorker).unwrap();

  let response = local_exchange.next_response().unwrap();
  assert_matches!(
    response.unwrap(),
    ResponseMessage::Feedback(Feedback::Status { .. })
  );
}
