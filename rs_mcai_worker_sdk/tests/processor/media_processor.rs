use assert_matches::assert_matches;
use mcai_worker_sdk::prelude::*;
use std::sync::{mpsc::Sender, Arc, Mutex};

#[test]
fn processor() {
  env_logger::init();

  let file_path = "./test_media_processor.mxf";
  let nb_frames = 500;
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

    fn init_process(
      &mut self,
      _parameters: WorkerParameters,
      format_context: Arc<Mutex<FormatContext>>,
      _result: Arc<Mutex<Sender<ProcessResult>>>,
    ) -> Result<Vec<StreamDescriptor>> {
      let mut stream_descriptors = vec![];

      let format_context = format_context.lock().unwrap();
      for stream_index in 0..format_context.get_nb_streams() {
        let stream_type = format_context.get_stream_type(stream_index as isize);
        info!(
          "Handle stream #{} with type: {:?}",
          stream_index, stream_type
        );

        match stream_type {
          AVMediaType::AVMEDIA_TYPE_VIDEO => {
            let filters = vec![VideoFilter::Resize(Scaling {
              width: Some(200),
              height: Some(70),
            })];
            stream_descriptors.push(StreamDescriptor::new_video(stream_index as usize, filters))
          }
          AVMediaType::AVMEDIA_TYPE_AUDIO => {
            let channel_layouts = vec!["mono".to_string()];
            let sample_formats = vec!["s16".to_string()];
            let sample_rates = vec![16000];

            let filters = vec![AudioFilter::Format(AudioFormat {
              sample_rates,
              channel_layouts,
              sample_formats,
            })];
            stream_descriptors.push(StreamDescriptor::new_audio(stream_index as usize, filters))
          }
          AVMediaType::AVMEDIA_TYPE_SUBTITLE => {
            stream_descriptors.push(StreamDescriptor::new_data(stream_index as usize))
          }
          AVMediaType::AVMEDIA_TYPE_DATA => {
            stream_descriptors.push(StreamDescriptor::new_data(stream_index as usize))
          }
          _ => info!("Skip stream #{}", stream_index),
        };
      }
      Ok(stream_descriptors)
    }

    fn process_frame(
      &mut self,
      _job_result: JobResult,
      _stream_index: usize,
      _frame: ProcessFrame,
    ) -> Result<ProcessResult> {
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
        "value": "./test_media_processor.json"
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

  let progress_messages = std::cmp::min(nb_frames - 1, 99);

  for _ in 0..progress_messages {
    let response = local_exchange.next_response().unwrap();
    log::debug!("{:?}", response);
    assert_matches!(
      response.unwrap(),
      ResponseMessage::Feedback(Feedback::Progression { .. })
    );
  }

  let response = local_exchange.next_response().unwrap();
  assert_matches!(response.unwrap(), ResponseMessage::Completed(_));

  // Second job, stop during execution
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

  local_exchange
    .send_order(OrderMessage::StopProcess(job.clone()))
    .unwrap();

  loop {
    let response = local_exchange.next_response().unwrap();
    match response {
      Some(ResponseMessage::Feedback(Feedback::Progression { .. })) => {}
      Some(ResponseMessage::JobStopped(JobResult { .. })) => {
        break;
      }
      _ => assert!(false),
    }
  }

  local_exchange.send_order(OrderMessage::StopWorker).unwrap();

  let response = local_exchange.next_response().unwrap();
  assert_matches!(
    response.unwrap(),
    ResponseMessage::Feedback(Feedback::Status { .. })
  );
}
