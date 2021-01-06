pub mod channels;
mod connection;
mod consumer;
mod current_orders;
mod exchange;
mod helpers;
mod publish;
mod publisher;

pub use connection::RabbitmqConnection;
pub use consumer::RabbitmqConsumer;
pub use current_orders::CurrentOrders;
pub use exchange::RabbitmqExchange;
pub use publisher::RabbitmqPublisher;

pub static EXCHANGE_NAME_JOB_RESPONSE: &str = "job_response";
pub static EXCHANGE_NAME_WORKER_RESPONSE: &str = "worker_response";

pub static EXCHANGE_NAME_SUBMIT: &str = "job_submit";
pub static EXCHANGE_NAME_DELAYED: &str = "job_delayed";
pub static EXCHANGE_NAME_DIRECT_MESSAGING: &str = "direct_messaging";
pub static EXCHANGE_NAME_RESPONSE_DELAYED: &str = "job_response_delayed";

// Job response exchange queues
pub static QUEUE_JOB_COMPLETED: &str = "job_completed";
pub static QUEUE_JOB_ERROR: &str = "job_error";
pub static QUEUE_JOB_PROGRESSION: &str = "job_progression";

pub static QUEUE_WORKER_DISCOVERY: &str = "worker_discovery";

// Worker response exchange queues
pub static QUEUE_WORKER_CREATED: &str = "worker_created";
pub static QUEUE_WORKER_INITIALIZED: &str = "worker_initialized";
pub static QUEUE_WORKER_STARTED: &str = "worker_started";
pub static QUEUE_WORKER_UPDATED: &str = "worker_updated";
pub static QUEUE_WORKER_STATUS: &str = "worker_status";
pub static QUEUE_WORKER_TERMINATED: &str = "worker_terminated";

// Not found queue names
pub static JOB_QUEUE_NOT_FOUND: &str = "job_queue_not_found";
pub static JOB_RESPONSE_NOT_FOUND: &str = "job_response_not_found";
pub static WORKER_RESPONSE_NOT_FOUND: &str = "worker_response_not_found";
pub static DIRECT_MESSAGING_NOT_FOUND: &str = "direct_messaging_not_found";
