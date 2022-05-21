use std::{sync::Arc, time::Duration};

use actix::{Actor, Addr, Context, Handler, Message};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use async_nats::{self, Connection};
use chrono::Utc;
use cloudevents::{Data, EventBuilder, EventBuilderV10};
use nats_actor2::model::event::event::EventError;
use nats_actor2::model::event::nats::pong::EVENT_TYPE_PONG;
use nats_actor2::subscriber::NatsStreamMessage;
use nats_actor2::{
    model::event::nats::{
        ping::{PingMessage, EVENT_TYPE_PING},
        pong::PongMessage,
    },
    subscriber::{subscribe, NatsSubscriberConfig},
    EventMessage, NatsClientSettings,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Message)]
#[rtype(result = "Result<bool, std::io::Error>")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MyLocalEvent {
    Ping(PingMessage),
    Pong(PongMessage),
}

impl TryFrom<MyLocalEvent> for cloudevents::Event {
    type Error = EventError;

    fn try_from(value: MyLocalEvent) -> Result<cloudevents::Event, Self::Error> {
        let builder = cloudevents::event::EventBuilderV10::new()
            .source("http://localhost")
            .time(Utc::now());

        let payload = json!(value);
        let builder = match value {
            MyLocalEvent::Ping(PingMessage { trace_id, message }) => builder
                .subject("ping_message")
                .ty(EVENT_TYPE_PING)
                .id(trace_id)
                .data(mime::APPLICATION_JSON.to_string(), payload),
            MyLocalEvent::Pong(PongMessage { trace_id, user_id }) => builder
                .subject("ping_message")
                .ty(EVENT_TYPE_PONG)
                .id(trace_id)
                .data(mime::APPLICATION_JSON.to_string(), payload),
        };

        builder.build().map_err(EventError::Builder)
    }
}

impl TryFrom<cloudevents::Event> for MyLocalEvent {
    type Error = EventError;

    fn try_from(event: cloudevents::Event) -> Result<MyLocalEvent, Self::Error> {
        event
            .data()
            .and_then(|data| match data {
                Data::Json(json) => serde_json::from_value(json.clone()).ok(),
                _ => None,
            })
            .and_then(|data| match serde_json::from_value::<MyLocalEvent>(data) {
                Ok(e) => Some(e),
                _ => None,
            })
            .ok_or_else(|| EventError::Parse("Missing or unrecognized event payload".into()))
    }
}

// Define actor
pub struct EventStreamHandler;

// Provide Actor implementation for our actor
impl Actor for EventStreamHandler {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        println!("Actor is alive");
    }

    fn stopped(&mut self, ctx: &mut Context<Self>) {
        println!("Actor is stopped");
    }
}

/// Define handler for `Ping` message
impl Handler<PingMessage> for EventStreamHandler {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: PingMessage, ctx: &mut Context<Self>) -> Self::Result {
        println!("Ping received: {:?}", msg);

        Ok(true)
    }
}

/// Define handler for `Pong` message
impl Handler<PongMessage> for EventStreamHandler {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: PongMessage, ctx: &mut Context<Self>) -> Self::Result {
        println!("Pong received: {:?}", msg);

        Ok(true)
    }
}

/// Define handler for `Event` message
impl Handler<MyLocalEvent> for EventStreamHandler {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: MyLocalEvent, ctx: &mut Context<Self>) -> Self::Result {
        println!("MyEnum received: {:?}", msg);

        Ok(true)
    }
}

// This struct represents state
#[derive(Clone)]
struct AppState {}

#[get("/hello")]
async fn hello(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().body("Hello!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let nats_address = format!("127.0.0.1:{}", 4222);
    let subject = "test_subject";

    let nats_stream_handler = EventStreamHandler.start();

    actix::spawn(async move {
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
            move |msg: NatsStreamMessage| {
                println!("Received event {:?}", msg);
                let event: cloudevents::Event = serde_json::from_slice(&msg.msg.data).unwrap();
                let event: MyLocalEvent = event.try_into().unwrap();
                println!("Extracted event {:?}", event);
                nats_stream_handler.try_send(event);
                Ok(())
            },
        )
        .await
        .unwrap();
    });

    let data = web::Data::new(AppState {});

    HttpServer::new(move || App::new().app_data(data.clone()).service(hello))
        .bind("127.0.0.1:8001")?
        .run()
        .await
}

// http://127.0.0.1:8001/hello
