use nats_actor2::model::event::event::Event;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let nats_address = format!("127.0.0.1:{}", 4222);
    let subject = "test_subject";

    let nc = async_nats::connect(&nats_address).await?;

    let sub = nc.subscribe(&subject).await?;

    loop {
        let e = sub.next().await.map(move |msg| {
            let e: cloudevents::Event = serde_json::from_slice(&msg.data).unwrap();
            let e: Event = e.try_into().unwrap();
            e
        });
        println!("received {:?}", e);
    }

    Ok(())
}
