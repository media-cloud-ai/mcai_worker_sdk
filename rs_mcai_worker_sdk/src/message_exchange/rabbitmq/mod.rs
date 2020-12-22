pub mod channels;
mod connection;
mod consumer;
mod exchange;
mod helpers;
mod publish;
mod publisher;

pub use connection::RabbitmqConnection;
pub use consumer::RabbitmqConsumer;
pub use exchange::RabbitmqExchange;
pub use publisher::RabbitmqPublisher;

pub static EXCHANGE_NAME_JOB_RESPONSE: &str = "job_response";
pub static EXCHANGE_NAME_WORKER_RESPONSE: &str = "worker_response";

pub static EXCHANGE_NAME_SUBMIT: &str = "job_submit";
pub static EXCHANGE_NAME_DELAYED: &str = "job_delayed";
pub static EXCHANGE_NAME_DIRECT_MESSAGING: &str = "direct_messaging";
pub static EXCHANGE_NAME_RESPONSE_DELAYED: &str = "job_response_delayed";

// Job response exchange routing keys
pub static ROUTING_KEY_JOB_COMPLETED: &str = "job_completed";
pub static ROUTING_KEY_JOB_ERROR: &str = "job_error";
pub static ROUTING_KEY_JOB_PROGRESSION: &str = "job_progression";

pub static QUEUE_WORKER_DISCOVERY: &str = "worker_discovery";

// Worker response exchange routing keys
pub static ROUTING_KEY_WORKER_CREATED: &str = "worker_created";
pub static ROUTING_KEY_WORKER_INITIALIZED: &str = "worker_initialized";
pub static ROUTING_KEY_WORKER_STARTED: &str = "worker_started";
pub static ROUTING_KEY_WORKER_UPDATED: &str = "worker_updated";
pub static ROUTING_KEY_WORKER_STATUS: &str = "worker_status";
pub static ROUTING_KEY_WORKER_TERMINATED: &str = "worker_terminated";

pub static NOT_FOUND_WORKER_QUEUE: &str = "worker_queue_not_found";
