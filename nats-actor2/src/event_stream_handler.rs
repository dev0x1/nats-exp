extern crate actix;
extern crate actix_rt;
use crate::model::event::nats::{ping::PingMessage, pong::PongMessage};
use actix::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Message)]
#[rtype(result = "Result<bool, std::io::Error>")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MyLocalEvent {
    Ping(PingMessage),
    Pong(PongMessage),
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

/// Define handler for `Pong` message
impl Handler<MyLocalEvent> for EventStreamHandler {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: MyLocalEvent, ctx: &mut Context<Self>) -> Self::Result {
        println!("MyEnum received: {:?}", msg);

        Ok(true)
    }
}
