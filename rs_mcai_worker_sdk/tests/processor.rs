#[macro_use]
#[cfg(not(feature = "media"))]
extern crate serde_derive;

#[test]
#[cfg(not(feature = "media"))]
fn processor() {
  use mcai_worker_sdk::message_exchange::ResponseMessage;
  use mcai_worker_sdk::{
    job::{Job, JobResult},
    message_exchange::{ExternalExchange, LocalExchange, OrderMessage},
    processor::Processor,
    JsonSchema, McaiChannel, MessageEvent, Result,
  };
  use std::sync::{Arc, Mutex};

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
      println!("Initialize processor test worker!");
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
  let worker = Arc::new(Mutex::new(worker));

  let exchange = local_exchange.clone();
  async_std::task::spawn(async move {
    let processor = Processor::new(exchange);
    assert!(processor.run(worker).is_ok());
  });

  let job = Job::new(r#"{ "job_id": 666, "parameters": [] }"#).unwrap();

  let local_exchange = Arc::make_mut(&mut local_exchange);
  local_exchange
    .send_order(OrderMessage::InitProcess(job.clone()))
    .unwrap();

  local_exchange
    .send_order(OrderMessage::StartProcess(job.clone()))
    .unwrap();

  local_exchange
    .send_order(OrderMessage::StopProcess(job.clone()))
    .unwrap();

  local_exchange.send_order(OrderMessage::StopWorker).unwrap();

  let expected_job_result = JobResult::from(job).with_message("OK");

  let response = local_exchange.next_response().unwrap();
  assert_eq!(ResponseMessage::Initialized, response.unwrap());

  let response = local_exchange.next_response().unwrap();
  assert_eq!(
    ResponseMessage::Completed(expected_job_result),
    response.unwrap()
  );
}

// #[cfg(feature = "media")]
// #[test]
// fn media_processor() {
//   use mcai_worker_sdk::{
//     processor::media::MediaProcessor, FormatContext, ProcessFrame, ProcessResult, StreamDescriptor,
//   };
//   use std::sync::mpsc::Sender;
//
//   struct Worker {}
//
//   #[derive(Clone, Debug, Deserialize, JsonSchema)]
//   pub struct WorkerParameters {
//     source_path: String,
//     destination_path: String,
//   }
//
//   impl MessageEvent<WorkerParameters> for Worker {
//     fn get_name(&self) -> String {
//       "Test Worker".to_string()
//     }
//
//     fn get_short_description(&self) -> String {
//       "The Worker defined in unit tests".to_string()
//     }
//
//     fn get_description(&self) -> String {
//       "Mock a Worker to realise tests around SDK".to_string()
//     }
//
//     fn get_version(&self) -> semver::Version {
//       semver::Version::parse("1.2.3").unwrap()
//     }
//
//     fn init_process(
//       &mut self,
//       _parameters: WorkerParameters,
//       _format_context: Arc<Mutex<FormatContext>>,
//       _response_sender: Arc<Mutex<Sender<ProcessResult>>>,
//     ) -> Result<Vec<StreamDescriptor>> {
//       println!("Init process!");
//       Ok(vec![StreamDescriptor::new_video(0, vec![])])
//     }
//
//     fn process_frame(
//       &mut self,
//       _job_result: JobResult,
//       stream_index: usize,
//       frame: ProcessFrame,
//     ) -> Result<ProcessResult> {
//       println!("Process stream {} frame!", stream_index);
//       match frame {
//         ProcessFrame::AudioVideo(frame) => {
//           unsafe {
//             let width = (*frame.frame).width;
//             let height = (*frame.frame).height;
//             let sample_rate = (*frame.frame).sample_rate;
//             let channels = (*frame.frame).channels;
//             let nb_samples = (*frame.frame).nb_samples;
//
//             if width != 0 && height != 0 {
//               println!("PTS: {}, image size: {}x{}", frame.get_pts(), width, height);
//             } else {
//               println!(
//                 "PTS: {}, sample_rate: {}Hz, channels: {}, nb_samples: {}",
//                 frame.get_pts(),
//                 sample_rate,
//                 channels,
//                 nb_samples,
//               );
//             }
//           }
//           println!("Return process result...");
//           return Ok(ProcessResult::empty());
//         }
//         _ => Ok(ProcessResult::empty()),
//       }
//     }
//
//     fn ending_process(&mut self) -> Result<()> {
//       println!("Ending process!");
//       Ok(())
//     }
//   }
//
//   let mut local_exchange = LocalExchange::new();
//   let local_exchange_ref = Arc::new(Mutex::new(local_exchange.clone()));
//   let processor = MediaProcessor::new(local_exchange_ref);
//
//   let worker = Worker {};
//
//   let join_handle = std::thread::spawn(move || {
//     let run_result = processor.run(worker);
//     println!("run_result: {:?}", run_result);
//     assert!(run_result.is_ok());
//   });
//
//   let job = Job::new(
//     r#"{
//   "job_id": 666,
//   "parameters": [
//     { "id": "source_path", "type": "string", "value": "/home/valentin/Vidéos/191496242-5bd703996a1d5-standard5_1s.mp4" },
//     { "id": "destination_path", "type": "string", "value": "./output.mxf" }
//    ]
//   }"#,
//   )
//   .unwrap();
//
//   println!("Send init...");
//   local_exchange
//     .send_order(OrderMessage::InitProcess(job.clone()))
//     .unwrap();
//   let response = local_exchange.next_response().unwrap();
//   println!("Init response: {:?}", response);
//
//   println!("Send process...");
//   local_exchange
//     .send_order(OrderMessage::StartProcess(job.clone()))
//     .unwrap();
//   println!("Wait for process response...");
//   let response = local_exchange.next_response().unwrap();
//   println!("Process response: {:?}", response);
//
//   println!("Send stop...");
//   local_exchange
//     .send_order(OrderMessage::StopProcess(job.clone()))
//     .unwrap();
//   let response = local_exchange.next_response().unwrap();
//   println!("Stop response: {:?}", response);
//
//   let result = join_handle.join();
//   println!("Thread joined! {:?}", result);
// }
