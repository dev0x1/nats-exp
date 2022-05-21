use std::time::Duration;

use actix::prelude::Message;
use async_nats::{Connection, Options};
use backoff::ExponentialBackoff;
use backoff::{backoff::Backoff, future::retry};

use chrono::prelude::Local;
use cloudevents::{Event as CloudEvent, EventBuilder, EventBuilderV10};
use derive_more::{Display, Error};
use serde::{Deserialize, Deserializer, Serialize};
use tracing::{debug, error, info, warn};

const NATS_CONNECTION_RETRY_INTERVAL_SECS: u64 = 10;

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

#[derive(Message, Debug)]
#[rtype(result = "Result<(), InternalError>")]
pub struct EventMessage {
    pub event: CloudEvent,
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

pub mod event_stream_handler;
pub mod model;
pub mod publisher;
pub mod subscriber;

#[cfg(test)]
mod tests {
    use actix::Actor;
    use cloudevents::{EventBuilder, EventBuilderV10};
    use serde_json::json;
    use serial_test::serial;
    use std::time::Duration;
    use uuid::Uuid;

    use crate::{
        event_stream_handler::{EventStreamHandler, MyLocalEvent},
        model::event::nats::{ping::PingMessage, pong::PongMessage},
        publisher::{NatsPublisher, NatsPublisherConfig},
        subscriber::{subscribe, NatsSubscriberConfig},
        EventMessage, NatsClientSettings,
    };

    #[actix_rt::test]
    #[serial]
    async fn should_publish_to_nats() {
        let nats_address = format!("127.0.0.1:{}", 4222);

        let uuid = Uuid::new_v4();
        let payload = json!({"user": "Ram", "loc": "India"});

        let event = EventBuilderV10::new()
            .source("http://localhost")
            .id(uuid.to_hyphenated().to_string())
            .subject("greetings")
            .ty("com.example.hello")
            .data("application/json", payload)
            .build()
            .unwrap();

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

    #[actix_rt::test]
    #[serial]
    async fn test_nominal() {
        // Start MyActor in current thread
        let addr = EventStreamHandler.start();

        // Send Ping message.
        // send() message returns Future object, that resolves to message result
        let result = addr
            .send(PingMessage {
                trace_id: "trace_ping".into(),
                message: "ping".into(),
            })
            .await;

        match result {
            Ok(res) => println!("Got result: {}", res.unwrap()),
            Err(err) => println!("Got error: {}", err),
        }

        // Send Pong message.
        // send() message returns Future object, that resolves to message result
        let result = addr
            .send(PongMessage {
                trace_id: "trace_pong".into(),
                user_id: 12345,
            })
            .await;

        match result {
            Ok(res) => println!("Got result: {}", res.unwrap()),
            Err(err) => println!("Got error: {}", err),
        }

        // Send Ping message wrapped in MyLocalEvent.
        // send() message returns Future object, that resolves to message result
        let result = addr
            .send(MyLocalEvent::Ping(PingMessage {
                trace_id: "trace_ping".into(),
                message: "ping".into(),
            }))
            .await;

        match result {
            Ok(res) => println!("Got result: {}", res.unwrap()),
            Err(err) => println!("Got error: {}", err),
        }
    }
}
