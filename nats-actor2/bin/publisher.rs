use std::{sync::Arc, time::Duration};

use actix::Addr;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use async_nats::{self, Connection};
use cloudevents::{EventBuilder, EventBuilderV10};
use nats_actor2::{
    model::event::{
        event::Event,
        nats::{
            ping::{PingMessage, EVENT_TYPE_PING},
            pong::{PongMessage, EVENT_TYPE_PONG},
        },
    },
    publisher::{NatsPublisher, NatsPublisherConfig},
    subscriber::{subscribe, NatsSubscriberConfig},
    EventMessage, NatsClientSettings,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

// This struct represents state
#[derive(Clone)]
struct AppState {
    nats_publisher: Arc<Addr<NatsPublisher>>,
}

#[get("/ping")]
async fn ping(data: web::Data<AppState>) -> impl Responder {
    let uuid = Uuid::new_v4();
    let ping_event = Event::Ping(PingMessage {
        trace_id: "123".to_owned(),
        message: "ping".to_owned(),
    });
    let payload = json!(ping_event);

    let event = EventBuilderV10::new()
        .source("http://localhost")
        .id(uuid.to_hyphenated().to_string())
        .subject("test_subject")
        .ty(EVENT_TYPE_PING)
        .data("application/json", payload)
        .build()
        .unwrap();

    let publisher = Arc::clone(&data.nats_publisher);
    publisher.do_send(EventMessage {
        event: ping_event.try_into().unwrap(),
    });

    HttpResponse::Ok().body("Ping!")
}

#[get("/pong")]
async fn pong(data: web::Data<AppState>) -> impl Responder {
    let uuid = Uuid::new_v4();
    let pong_event = Event::Pong(PongMessage {
        trace_id: "100".to_owned(),
        user_id: 1122334455,
    });
    let payload = json!(pong_event);

    let event = EventBuilderV10::new()
        .source("http://localhost")
        .id(uuid.to_hyphenated().to_string())
        .subject("test_subject")
        .ty(EVENT_TYPE_PONG)
        .data("application/json", payload)
        .build()
        .unwrap();

    let publisher = Arc::clone(&data.nats_publisher);
    publisher.do_send(EventMessage {
        event: pong_event.try_into().unwrap(),
    });

    HttpResponse::Ok().body("Pong!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let nats_address = format!("127.0.0.1:{}", 4222);
    let subject = "test_subject";

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

    let data = web::Data::new(AppState {
        nats_publisher: Arc::new(publisher),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .service(ping)
            .service(pong)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}

// http://127.0.0.1:8000/ping
