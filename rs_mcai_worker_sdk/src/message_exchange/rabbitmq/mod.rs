mod connection;
mod consumer;
mod exchange;
mod helpers;
mod publish;

pub use connection::RabbitmqConnection;
pub use consumer::RabbitmqConsumer;
pub use exchange::RabbitmqExchange;

pub static RESPONSE_EXCHANGE: &str = "job_response";
pub static QUEUE_JOB_COMPLETED: &str = "job_completed";
pub static QUEUE_JOB_ERROR: &str = "job_error";
pub static QUEUE_JOB_PROGRESSION: &str = "job_progression";
