use mcai_worker_sdk::prelude::*;
use std::sync::{mpsc::{self, Sender}, Arc, Mutex};

#[async_std::test]
async fn processor() -> Result<()> {
  env_logger::init();

  let file_path = "./test_rabbitmq_media_processor.mxf";
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
  let (initialized_sender, initialized_receiver) = mpsc::channel::<JobResult>();
  let (started_sender, started_receiver) = mpsc::channel::<JobResult>();

  let (progression_sender, progression_receiver) = mpsc::channel::<JobProgression>();
  let (stopped_sender, stopped_receiver) = mpsc::channel::<JobResult>();

  let amqp_connection = super::AmqpConnection::new().unwrap();

  amqp_connection.start_consumer(QUEUE_WORKER_CREATED, created_sender);
  amqp_connection.start_consumer(QUEUE_WORKER_STATUS, status_sender);
  amqp_connection.start_consumer(QUEUE_WORKER_INITIALIZED, initialized_sender);
  amqp_connection.start_consumer(QUEUE_WORKER_STARTED, started_sender);

  amqp_connection.start_consumer(QUEUE_JOB_PROGRESSION, progression_sender);
  amqp_connection.start_consumer(QUEUE_JOB_STOPPED, stopped_sender);

  assert!(created_receiver.recv().is_ok());

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::Status)?;
  assert!(status_receiver.recv().is_ok());

  let job = Job::new(
    r#"{
    "job_id": 999,
    "parameters": [
      {
        "id": "source_path",
        "type": "string",
        "value": "./test_rabbitmq_media_processor.mxf"
      },
      {
        "id": "destination_path",
        "type": "string",
        "value": "./test_rabbitmq_media_processor.json"
      }
    ]
  }"#,
  )
  .unwrap();

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::InitProcess(job.clone()))?;
  assert!(initialized_receiver.recv().is_ok());

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::StartProcess(job.clone()))?;

  assert!(started_receiver.recv().is_ok());
  assert!(progression_receiver.recv().is_ok());

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::StopProcess(job.clone()))?;
  let stopped_message = stopped_receiver.recv();
  assert!(stopped_message.is_ok());

  std::thread::sleep(std::time::Duration::from_millis(2000));

  log::error!("Get the status of the worker");

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::Status)?;
  assert!(status_receiver.recv().is_ok());

  log::error!("Second time same job");

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::InitProcess(job.clone()))?;
  assert!(initialized_receiver.recv().is_ok());

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::StartProcess(job.clone()))?;

  assert!(started_receiver.recv().is_ok());
  assert!(progression_receiver.recv().is_ok());

  amqp_connection.send_order(vec!["worker_id"], &OrderMessage::StopProcess(job))?;
  let stopped_message = stopped_receiver.recv();
  assert!(stopped_message.is_ok());

  std::thread::sleep(std::time::Duration::from_millis(2000));

  Ok(())
}
