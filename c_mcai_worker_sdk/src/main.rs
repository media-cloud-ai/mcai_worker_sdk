use c_mcai_worker_sdk::worker::CWorkerEvent;
use mcai_worker_sdk::prelude::*;

fn main() {
  start_worker(CWorkerEvent::default());
}
