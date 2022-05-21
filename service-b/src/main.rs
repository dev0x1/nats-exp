use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Person {
    first_name: String,
    last_name: String,
    age: u8,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let nc = async_nats::connect("demo.nats.io").await?;
    let subj = "hellooo".to_owned();

    let p = Person {
        first_name: "derek".to_owned(),
        last_name: "collison".to_owned(),
        age: 22,
    };

    let sub = nc.subscribe(&subj).await?;

    loop {
        let p = sub.next().await.map(move |msg| {
            let p: Person = serde_json::from_slice(&msg.data).unwrap();
            p
        });
        println!("received {:?}", p);
    }

    Ok(())
}
