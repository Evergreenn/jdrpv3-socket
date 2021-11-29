#[macro_use]
extern crate dotenv_codegen;

mod claim;
mod redis_c;

use redis_c::*;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use ws::{
    listen,
    CloseCode,
    Error,
    Handler,
    Handshake,
    Message,
    //  Request, Response,
    Result,
    Sender,
};
// use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
struct Room {
    items: Vec<String>,
}

struct Server {
    out: Sender,
    master: String,
    user: String,
}

#[derive(Serialize, Deserialize)]
pub struct Messages {
    pub message: String,
    pub from: String,
    // pub date: Option<SystemTime>,
    // pub room_date: Option<SystemTime>,
}

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(short = "w", long = "websocket")]
    ws: String,
    #[structopt(short = "t", long = "jwt")]
    jwt: String,
}

impl Handler for Server {
    fn on_open(&mut self, hs: Handshake) -> Result<()> {
        // We have a new connection, so we increment the connection counter
        // Ok(self.count.set(self.count.get() + 1))

        let response = Messages {
            message: format!("game created by: {}", self.master),
            from: "Admin".into(),
        };
        let stringify = serde_json::to_string(&response).unwrap();
        self.out.send(stringify).unwrap();

        let jwt_token = hs.request.resource().split("=").collect::<Vec<_>>()[1];
        let jwt_struct = claim::decode_jwt(&jwt_token).unwrap();

        // hs.request.header(header: &str)
        self.user = jwt_struct.username;
        let co = redis_co::get_redis_co();

        redis_co::get_set_h_to_redis(co, String::from(&*self.user));

        // println!("{:#?}", String::from(&*self.user));
        // println!("{:#?}", self.out.connection_id());
        let response = Messages {
            message: format!("{} is connected", self.user),
            from: "Admin".into(),
        };

        let stringify = serde_json::to_string(&response).unwrap();

        self.out.broadcast(stringify).unwrap();

        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let raw_message = msg.into_text()?;

        let response = Messages {
            message: raw_message,
            from: "User".into(),
        };

        let stringify = serde_json::to_string(&response).unwrap();

        // Echo the to all
        self.out.broadcast(stringify)
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        // The WebSocket protocol allows for a utf8 reason for the closing state after the
        // close code. WS-RS will attempt to interpret this data as a utf8 description of the
        // reason for closing the connection. I many cases, `reason` will be an empty string.
        // So, you may not normally want to display `reason` to the user,
        // but let's assume that we know that `reason` is human-readable.
        let _ = match code {
            CloseCode::Normal => println!("The client is done with the connection."),
            CloseCode::Away => println!("The client is leaving the site."),
            CloseCode::Abnormal => {
                println!("Closing handshake failed! Unable to obtain closing status from client.")
            }
            _ => println!("The client encountered an error: {}", reason),
        };
        let co = redis_co::get_redis_co();
        redis_co::remove_h_to_redis(co, String::from(&*self.user));

        let response = Messages {
            message: format!("{} is deconnected", self.user),
            from: "Admin".into(),
        };

        let stringify = serde_json::to_string(&response).unwrap();

        self.out.broadcast(stringify).unwrap();


    }

    fn on_error(&mut self, err: Error) {
        println!("The server encountered an error: {:?}", err);

        // The connection is going down, so we need to decrement the count
    }
}

// Now, instead of a closure, the Factory returns a new instance of our Handler.
fn main() {
    let args = Args::from_args();
    println!("{:#?}", args);

    let addr = args.ws;
    // let jwt = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6IlllaHJhIiwicGVybWlzc2lvbnMiOlsiT1BfR0VUX1NFQ1VSRURfSU5GTyIsIlJPTEVfVVNFUiJdLCJleHAiOjE2MzgyNTgxNDh9.hhI09t_A29HxiaBTkaxTD1wp-u8uP0DljRW5F_exaPI";
    let jwt = args.jwt;
    let jwt_struct = claim::decode_jwt(&jwt).unwrap();

    let r = Room { items: vec![] };
    let to_store = serde_json::to_string(&r).unwrap();

    let mut co = redis_co::get_redis_co();
    let _: () = redis::cmd("HSET")
        .arg("ROOM")
        .arg("people")
        .arg(to_store)
        .query(&mut co)
        .unwrap();

    listen(addr, |out| Server {
        out: out,
        master: jwt_struct.username.clone(),
        user: String::from(""),
    })
    .unwrap()
}
