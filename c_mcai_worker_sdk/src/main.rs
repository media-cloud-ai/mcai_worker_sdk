use c_mcai_worker_sdk::worker::CWorkerEvent;
use mcai_worker_sdk::start_worker;

fn main() {
  start_worker(CWorkerEvent::default());
}
