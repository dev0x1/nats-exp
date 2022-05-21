use std::{sync::Arc, time::Duration};

use actix::Addr;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use async_nats::{self, Connection};
use nats_actor::{
    publisher::{NatsPublisher, NatsPublisherConfig},
    subscriber::{subscribe, NatsSubscriberConfig},
    Event, EventMessage, NatsClientSettings,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Person {
    first_name: String,
    last_name: String,
    age: u8,
}

// This struct represents state
#[derive(Clone)]
struct AppState {
    nats_publisher: Arc<Addr<NatsPublisher>>,
}

#[get("/hello")]
async fn hello(data: web::Data<AppState>) -> impl Responder {
    let event = Event::new(format!("event_type_{}", 12345));

    let publisher = Arc::clone(&data.nats_publisher);
    publisher.do_send(EventMessage {
        event: event.clone(),
    });

    HttpResponse::Ok().body("Hello!")
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

    HttpServer::new(move || App::new().app_data(data.clone()).service(hello))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

// http://127.0.0.1:8080/hello
