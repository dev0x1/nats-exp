use std::time::Duration;

use actix::prelude::Message;
use async_nats::{Connection, Options};
use backoff::ExponentialBackoff;
use backoff::{backoff::Backoff, future::retry};

use chrono::prelude::Local;
use derive_more::{Display, Error};
use serde::{Deserialize, Deserializer, Serialize};
use tracing::{debug, error, info, warn};

#[derive(Clone, Debug, Display, Error)]
pub enum InternalError {
    #[display(fmt = "Nats server connection failed: {address}")]
    NatsServerConnectionError { address: String },
    #[display(fmt = "Nats operation failed: {cause}")]
    NatsOperationError { cause: String },
    #[display(fmt = "Failed while ser-de Nats message {cause}")]
    SerdeError { cause: String },
    #[display(fmt = "Error: {}", cause)]
    GenericError { cause: String },
}

/// An Event is published from a service to other services using NATS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Event {
    #[serde(default = "default_trace_id")]
    #[serde(deserialize_with = "deserialize_null_trace_id")]
    pub trace_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub created_ms: u64,
    pub payload: Payload,
}

impl Event {
    pub fn new<S: Into<String>>(event_type: S) -> Event {
        Event::new_with_payload(event_type, Map::new())
    }

    pub fn new_with_payload<S: Into<String>>(event_type: S, payload: Payload) -> Event {
        let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
        let created_ms = dt.timestamp_millis() as u64;
        Event {
            trace_id: default_trace_id(),
            event_type: event_type.into(),
            created_ms,
            payload,
        }
    }
}

pub type Payload = Map<String, Value>;
pub type Value = serde_json::Value;
pub type Map<K, V> = serde_json::Map<K, V>;
pub type Number = serde_json::Number;

#[inline]
fn default_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn deserialize_null_trace_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_else(default_trace_id))
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), InternalError>")]
pub struct EventMessage {
    pub event: Event,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NatsClientSettings {
    pub addresses: Vec<String>,
    pub max_reconnects: Option<usize>,
    pub retry_timeout: Option<Duration>,
}

///
/// Back-off policy for retry.
///
fn backoff(timeout: Option<Duration>) -> ExponentialBackoff {
    ExponentialBackoff {
        max_elapsed_time: timeout,
        ..Default::default()
    }
}

pub async fn connect_with_retry(config: &NatsClientSettings) -> Result<Connection, InternalError> {
    info!("Connecting to NATS...");
    let addresses = config.addresses.join(",");

    let connect_op = || async {
        let options = Options::new()
            .disconnect_callback(|| error!("connection lost"))
            .max_reconnects(config.max_reconnects);

        Ok(options.connect(&addresses).await?)
    };

    retry(backoff(config.retry_timeout), connect_op)
        .await
        .map_err(|_| InternalError::NatsServerConnectionError { address: addresses })
}

pub async fn connect(config: &NatsClientSettings) -> Result<Connection, InternalError> {
    info!("Connecting to NATS...");
    let addresses = config.addresses.join(",");

    let options = Options::new()
        .disconnect_callback(|| error!("connection lost"))
        .max_reconnects(config.max_reconnects);

    options
        .connect(&addresses)
        .await
        .map_err(|_| InternalError::NatsServerConnectionError { address: addresses })
}

pub mod publisher;
pub mod subscriber;

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::time::Duration;

    use crate::{
        publisher::{NatsPublisher, NatsPublisherConfig},
        subscriber::{subscribe, NatsSubscriberConfig},
        Event, EventMessage, NatsClientSettings,
    };

    #[actix_rt::test]
    #[serial]
    async fn should_publish_to_nats() {
        let nats_address = format!("127.0.0.1:{}", 4222);

        let event = Event::new(format!("event_type_{}", 12345));
        let subject = format!("test_subject_{}", 12345);

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        subscribe(
            NatsSubscriberConfig {
                client_settings: NatsClientSettings {
                    addresses: vec![nats_address.to_owned()],
                    max_reconnects: Some(5),
                    retry_timeout: Some(Duration::from_secs(30)),
                },
                subject: subject.to_owned(),
                mailbox_size: 100,
            },
            move |event| {
                sender.send(event).unwrap();
                Ok(())
            },
        )
        .await
        .unwrap();

        let publisher = NatsPublisher::start_new(NatsPublisherConfig {
            client_settings: NatsClientSettings {
                addresses: vec![nats_address.to_owned()],
                max_reconnects: Some(5),
                retry_timeout: Some(Duration::from_secs(30)),
            },
            subject: subject.to_owned(),
            mailbox_size: 100,
        })
        .await
        .unwrap();
        publisher.do_send(EventMessage {
            event: event.clone(),
        });

        assert_eq!(
            event,
            serde_json::from_slice(&receiver.recv().await.unwrap().msg.data).unwrap()
        );
    }
}
