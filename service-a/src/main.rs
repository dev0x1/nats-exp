use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use async_nats::{self, Connection};
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
    nats_conn: Connection,
    nats_sub: String,
}

#[get("/hello")]
async fn hello(data: web::Data<AppState>) -> impl Responder {
    let p = Person {
        first_name: "derek".to_owned(),
        last_name: "collison".to_owned(),
        age: 22,
    };

    let _ = &data
        .nats_conn
        .publish(&data.nats_sub, serde_json::to_vec(&p).unwrap())
        .await
        .unwrap();

    HttpResponse::Ok().body("Hello!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let nc = async_nats::connect("demo.nats.io").await?;
    //let subj = nc.new_inbox();
    let subj = "hellooo".to_owned();

    let data = web::Data::new(AppState {
        nats_conn: nc,
        nats_sub: subj,
    });

    HttpServer::new(move || App::new().app_data(data.clone()).service(hello))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

// http://127.0.0.1:8080/hello
