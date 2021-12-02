#[macro_use]
extern crate dotenv_codegen;

mod claim;
mod redis_c;

use chrono::Utc;
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
enum State {
    Log,
    Console,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            State::Log => write!(f, "Log"),
            State::Console => write!(f, "Console"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Messages {
    pub message: String,
    pub is_admin: bool,
    pub from: String,
    pub date: i64,
    // pub room_date: Option<SystemTime>,
}

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(short = "w", long = "websocket")]
    ws: String,
    #[structopt(short = "t", long = "jwt")]
    jwt: String,
}

pub fn create_message(i: &Sender, msg: String, from: String, is_broadcast: bool) -> Result<()> {
    let response = Messages {
        message: msg,
        is_admin: false,
        from: from,
        date: Utc::now().timestamp_millis(),
    };
    let resp = serde_json::to_string(&response).unwrap();

    match is_broadcast {
        true => i.broadcast(resp),
        false => i.send(resp),
    }
}

pub fn create_log_message(i: &Sender, msg: String, is_broadcast: bool) -> Result<()> {
    let response = Messages {
        message: msg,
        is_admin: true,
        from: State::Log.to_string(),
        date: Utc::now().timestamp_millis(),
    };
    let resp = serde_json::to_string(&response).unwrap();

    match is_broadcast {
        true => i.broadcast(resp),
        false => i.send(resp),
    }
}

pub fn create_console_message(i: &Sender, msg: String) -> Result<()> {
    let response = Messages {
        message: msg,
        is_admin: true,
        from: State::Console.to_string(),
        date: Utc::now().timestamp_millis(),
    };
    let resp = serde_json::to_string(&response).unwrap();

    i.send(resp)
}

impl Handler for Server {
    fn on_open(&mut self, hs: Handshake) -> Result<()> {
        create_log_message(
            &self.out,
            format!("game created by: {}", self.master),
            false,
        )
        .unwrap();

        let jwt_token = hs.request.resource().split("=").collect::<Vec<_>>()[1];
        let jwt_struct = claim::decode_jwt(&jwt_token).unwrap();

        if jwt_struct.username == self.master {
            create_console_message(&self.out, String::from("")).unwrap();
        }

        // hs.request.header(header: &str)
        self.user = jwt_struct.username;
        let co = redis_co::get_redis_co();
        redis_co::get_set_h_to_redis(co, String::from(&*self.user));

        create_log_message(&self.out, format!("{} is connected", self.user), true).unwrap();

        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let raw_message = msg.into_text()?;

        create_message(&self.out, raw_message, self.user.to_string(), true)
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

        create_log_message(&self.out, format!("{} is deconnected", self.user), true).unwrap();
    }

    fn on_error(&mut self, err: Error) {
        println!("The server encountered an error: {:?}", err);
    }
}

fn main() {
    let args = Args::from_args();
    println!("{:#?}", args);

    let addr = args.ws;
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
