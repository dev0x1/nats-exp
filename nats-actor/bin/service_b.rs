use nats_actor::Event;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Person {
    first_name: String,
    last_name: String,
    age: u8,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let nats_address = format!("127.0.0.1:{}", 4222);
    let subject = "test_subject";

    let nc = async_nats::connect(&nats_address).await?;

    let p = Person {
        first_name: "derek".to_owned(),
        last_name: "collison".to_owned(),
        age: 22,
    };

    let sub = nc.subscribe(&subject).await?;

    loop {
        let e = sub.next().await.map(move |msg| {
            let e: Event = serde_json::from_slice(&msg.data).unwrap();
            e
        });
        println!("received {:?}", e);
    }

    Ok(())
}
