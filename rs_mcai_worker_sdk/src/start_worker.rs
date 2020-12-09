use chrono::prelude::*;
use crate::{
  config::*,
  job::Job,
  message_exchange::{
    ExternalExchange, LocalExchange, OrderMessage, RabbitmqExchange, ResponseMessage,
  },
  worker::{docker, WorkerConfiguration},
  MessageEvent,
  Processor,
};
use env_logger::Builder;
use futures_executor::LocalPool;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{
  fs,
  io::Write,
  str::FromStr,
  sync::{Arc, Mutex},
  thread, time,
};

/// Function to start a worker
pub fn start_worker<P: DeserializeOwned + JsonSchema, ME: 'static + MessageEvent<P>>(
  mut message_event: ME,
) where
  ME: std::marker::Sync + Send,
{
  let mut builder = Builder::from_default_env();
  let amqp_queue = get_amqp_queue();
  let instance_id = docker::get_instance_id("/proc/self/cgroup");

  let container_id = instance_id.clone();
  builder
    .format(move |stream, record| {
      writeln!(
        stream,
        "{} - {} - {} - {} - {} - {}",
        Utc::now(),
        &container_id,
        get_amqp_queue(),
        record.target().parse::<i64>().unwrap_or(-1),
        record.level(),
        record.args(),
      )
    })
    .init();

  let worker_configuration =
    WorkerConfiguration::new(&amqp_queue, &message_event, &instance_id);
  if let Err(configuration_error) = worker_configuration {
    error!("{:?}", configuration_error);
    return;
  }

  let worker_configuration = worker_configuration.unwrap();

  info!(
    "Worker: {}, version: {} (MCAI Worker SDK {})",
    worker_configuration.get_worker_name(),
    worker_configuration.get_worker_version(),
    worker_configuration.get_sdk_version(),
  );

  if let Ok(enabled) = std::env::var("DESCRIBE") {
    if enabled == "1" || bool::from_str(&enabled.to_lowercase()).unwrap_or(false) {
      match serde_json::to_string_pretty(&worker_configuration) {
        Ok(serialized_configuration) => {
          println!("{}", serialized_configuration);
          return;
        }
        Err(error) => error!("Could not serialize worker configuration: {:?}", error),
      }
    }
  }

  if let Err(message) = message_event.init() {
    error!("{:?}", message);
    return;
  }

  let shared_message_event = Arc::new(Mutex::new(message_event));
  info!("Worker initialized, ready to receive jobs");

  if let Some(source_orders) = get_source_orders() {
    warn!("Worker will process source orders");

    let exchange = LocalExchange::new();
    let shared_exchange = Arc::new(exchange);

    let cloned_exchange = shared_exchange.clone();
    async_std::task::spawn(async move {
      let processor = Processor::new(cloned_exchange);
      processor.run(shared_message_event.clone()).unwrap();
    });

    for source_order in &source_orders {
      info!("Start to process order: {:?}", source_order);

      let message_data = fs::read_to_string(source_order).unwrap();

      let job = Job::new(&message_data).unwrap();

      log::debug!(target: &job.job_id.to_string(),
        "received message: {:?}", job);

      let mut local_exchange = shared_exchange.clone();
      {
        let local_exchange = Arc::make_mut(&mut local_exchange);
        local_exchange
          .send_order(OrderMessage::InitProcess(job.clone()))
          .unwrap();
        local_exchange
          .send_order(OrderMessage::StartProcess(job))
          .unwrap();
      }

      let mut local_exchange = shared_exchange.clone();
      let local_exchange = Arc::make_mut(&mut local_exchange);

      while let Ok(message) = local_exchange.next_response() {
        info!("{:?}", message);
        match message {
          Some(ResponseMessage::Completed(_)) | Some(ResponseMessage::Error(_)) => {
            break;
          }
          _ => {}
        }
      }
    }

    return;
  }

  loop {
    let mut executor = LocalPool::new();

    executor.run_until(async {
      let mut exchange = RabbitmqExchange::new(&worker_configuration).await.unwrap();

      exchange
        .bind_consumer(&amqp_queue, "amqp_worker")
        .await
        .unwrap();

      exchange
        .bind_consumer(
          &worker_configuration.get_direct_messaging_queue_name(),
          "status_amqp_worker",
        )
        .await
        .unwrap();

      let exchange = Arc::new(exchange);

      let processor = Processor::new(exchange);

      processor.run(shared_message_event.clone()).unwrap();
    });

    let sleep_duration = time::Duration::new(1, 0);
    thread::sleep(sleep_duration);
    info!("Reconnection...");
  }
}
