use actix::Message;
use serde::{Deserialize, Serialize};

pub const EVENT_TYPE_PING: &str = "com.example.ping";

#[derive(Message)]
#[rtype(result = "Result<bool, std::io::Error>")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PingMessage {
    pub trace_id: String,
    pub message: String,
}
