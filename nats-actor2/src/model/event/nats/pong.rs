use actix::Message;
use serde::{Deserialize, Serialize};

pub const EVENT_TYPE_PONG: &str = "com.example.pong";

#[derive(Message)]
#[rtype(result = "Result<bool, std::io::Error>")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PongMessage {
    pub trace_id: String,
    pub user_id: u64,
}
