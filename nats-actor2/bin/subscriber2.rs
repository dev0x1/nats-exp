use std::{sync::Arc, time::Duration};

use actix::Addr;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use async_nats::{self, Connection};
use cloudevents::{EventBuilder, EventBuilderV10};
use nats_actor2::{
    model::event::{
        event::Event,
        nats::ping::{PingMessage, EVENT_TYPE_PING},
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

#[get("/hello")]
async fn hello(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().body("Hello!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let nats_address = format!("127.0.0.1:{}", 4222);
    let subject = "test_subject";

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
            move |event| {
                println!("Received event {:?}", event);
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
