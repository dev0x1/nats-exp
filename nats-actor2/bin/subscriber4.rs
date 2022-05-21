use std::{sync::Arc, time::Duration};

use actix::{Actor, Addr, Context, Handler, Message};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use async_nats::{self, Connection};
use chrono::Utc;
use cloudevents::{Data, EventBuilder, EventBuilderV10};
use nats_actor2::model::event::event::{Event, EventError};
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

// This struct represents state
#[derive(Clone)]
struct AppState {}

// Define actor
pub struct EventStreamHandler {
    context: web::Data<AppState>,
}

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
fn process_ping(event_message: PingMessage) {
    println!("Processing Ping...: {:?}", event_message);
}

/// Define handler for `Event` message
impl Handler<Event> for EventStreamHandler {
    type Result = Result<(), std::io::Error>;

    fn handle(&mut self, event: Event, ctx: &mut Context<Self>) -> Self::Result {
        println!("Handling event: {:?}", event);
        match event {
            Event::Ping(event_message) => process_ping(event_message),
            _ => {
                println!("Unprocessed event: {:?}", event);
            }
        }

        Ok(())
    }
}

#[get("/hello")]
async fn hello(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().body("Hello!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let nats_address = format!("127.0.0.1:{}", 4222);
    let subject = "test_subject";

    let state = web::Data::new(AppState {});

    let nats_stream_handler = EventStreamHandler {
        context: state.clone(),
    }
    .start();

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
                let event: Event = event.try_into().unwrap();
                println!("Extracted event {:?}", event);
                nats_stream_handler.try_send(event);
                Ok(())
            },
        )
        .await
        .unwrap();
    });

    HttpServer::new(move || App::new().app_data(state.clone()).service(hello))
        .bind("127.0.0.1:8001")?
        .run()
        .await
}

// http://127.0.0.1:8001/hello
