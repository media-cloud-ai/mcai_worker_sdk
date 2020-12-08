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
      assert!(channel.is_none());
      Ok(job_result.with_message("OK"))
    }
  }

  let mut local_exchange = LocalExchange::new();
  let local_exchange_ref = Arc::new(Mutex::new(local_exchange.clone()));
  let processor = Processor::new(local_exchange_ref);

  let worker = Worker {};

  std::thread::spawn(move || {
    assert!(processor.run(worker).is_ok());
  });

  let job = Job::new(r#"{ "job_id": 666, "parameters": [] }"#).unwrap();

  local_exchange
    .send_order(OrderMessage::StartProcess(job.clone()))
    .unwrap();
  local_exchange
    .send_order(OrderMessage::StopProcess(job.clone()))
    .unwrap();

  let expected_job_result = JobResult::from(job).with_message("OK");

  let response = local_exchange.next_response().unwrap();
  assert_eq!(
    ResponseMessage::Completed(expected_job_result),
    response.unwrap()
  );

}
