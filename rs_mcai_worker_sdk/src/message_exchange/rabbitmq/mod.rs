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

pub static EXCHANGE_NAME_RESPONSE: &str = "job_response";

pub static EXCHANGE_NAME_SUBMIT: &str = "job_submit";
pub static EXCHANGE_NAME_DELAYED: &str = "job_delayed";
pub static EXCHANGE_NAME_DIRECT_MESSAGING: &str = "direct_messaging";
pub static EXCHANGE_NAME_RESPONSE_DELAYED: &str = "job_response_delayed";

pub static ROUTING_KEY_JOB_STATUS: &str = "job_status";

pub static QUEUE_JOB_COMPLETED: &str = "job_completed";
pub static QUEUE_JOB_ERROR: &str = "job_error";
pub static QUEUE_JOB_PROGRESSION: &str = "job_progression";

pub static QUEUE_WORKER_DISCOVERY: &str = "worker_discovery";
