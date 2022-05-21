use actix::Message;
use chrono::Utc;
use cloudevents::{AttributesReader, Data, EventBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use super::nats::{
    ping::{PingMessage, EVENT_TYPE_PING},
    pong::{PongMessage, EVENT_TYPE_PONG},
};

#[derive(Debug, Error)]
pub enum EventError {
    #[error("Failed to parse event: {0}")]
    Parse(String),
    #[error("Failed to build event: {0}")]
    Builder(cloudevents::event::EventBuilderError),
    #[error("Failed to encode event payload: {0}")]
    PayloadEncoder(#[source] serde_json::Error),
    #[error("Unknown event type: {0}")]
    UnknownType(String),
    #[error("Cloud event error")]
    CloudEvent(cloudevents::message::Error),
    #[error("Failed to connect: {0}")]
    Connection(String),
    #[error("Failed to send: {0}")]
    Send(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Message)]
#[rtype(result = "Result<(), std::io::Error>")]
pub enum Event {
    Ping(PingMessage),
    Pong(PongMessage),
}

impl TryFrom<Event> for cloudevents::Event {
    type Error = EventError;

    fn try_from(value: Event) -> Result<cloudevents::Event, Self::Error> {
        let builder = cloudevents::event::EventBuilderV10::new()
            .source("http://localhost")
            .time(Utc::now());

        let payload = json!(value);
        let builder = match value {
            Event::Ping(PingMessage { trace_id, message }) => builder
                .subject("ping_message")
                .ty(EVENT_TYPE_PING)
                .id(trace_id)
                .data(mime::APPLICATION_JSON.to_string(), payload),
            Event::Pong(PongMessage { trace_id, user_id }) => builder
                .subject("ping_message")
                .ty(EVENT_TYPE_PONG)
                .id(trace_id)
                .data(mime::APPLICATION_JSON.to_string(), payload),
        };

        builder.build().map_err(EventError::Builder)
    }
}

impl TryFrom<cloudevents::Event> for Event {
    type Error = EventError;

    fn try_from(event: cloudevents::Event) -> Result<Event, Self::Error> {
        event
            .data()
            .and_then(|data| match data {
                Data::Json(json) => serde_json::from_value(json.clone()).ok(),
                _ => None,
            })
            .and_then(|data| match serde_json::from_value::<Event>(data) {
                Ok(e) => Some(e),
                _ => None,
            })
            .ok_or_else(|| EventError::Parse("Missing or unrecognized event payload".into()))
    }
}
