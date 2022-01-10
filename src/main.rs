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

fn main() {
    let args = Args::from_args();
    println!("{:#?}", args);

    let addr = args.ws;
    let jwt = args.jwt;
    let jwt_struct = claim::decode_jwt(&jwt).unwrap();

    // let r = Room { items: vec![] };
    // let to_store = serde_json::to_string(&r).unwrap();

    // let mut co = redis_co::get_redis_co();
    // let _: () = redis::cmd("HSET")
    //     .arg(args.game_id.clone())
    //     .arg("admin")
    //     .arg(to_store)
    //     .query(&mut co)
    //     .unwrap();

    listen(addr, |out| Server {
        out: out,
        master: jwt_struct.user_id.clone(),
        user: String::from(""),
        user_name: String::from(""),
        game_id: args.game_id.clone(),
    })
    .unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
struct Room {
    items: Vec<String>,
}

struct Server {
    out: Sender,
    master: String,
    user: String,
    user_name: String,
    game_id: String,
}

#[derive(Serialize, Deserialize)]
enum State {
    Log,
    Console,
    Admin,
    Game,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            State::Log => write!(f, "Log"),
            State::Console => write!(f, "Console"),
            State::Admin => write!(f, "Admin"),
            State::Game => write!(f, "Game"),
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

#[derive(Serialize, Deserialize)]
pub enum ConsoleScope {
    PlayerConnection,
    PlayerLeave,
}

#[derive(Serialize, Deserialize)]
pub struct ConsoleMessages {
    pub message: String,
    pub scope: ConsoleScope,
    pub from: String,
    // pub date: i64,
    // pub room_date: Option<SystemTime>,
}

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(short = "w", long = "websocket")]
    ws: String,
    #[structopt(short = "t", long = "jwt")]
    jwt: String,
    #[structopt(short = "g", long = "gameid")]
    game_id: String,
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

pub fn send_owner_console_message(i: &Sender, game_id: String, msg: String, scope: ConsoleScope) -> Result<()> {

    let co = redis_co::get_redis_co();

    let ids = redis_co::get_token_connectionid(
        game_id,
        String::from("admin"),
        co,
    );

    let response = ConsoleMessages {
        message: msg,
        from: State::Game.to_string(),
        scope: scope
    };
    let resp = serde_json::to_string(&response).unwrap();

    i.unicast(resp, ids.0, ids.1)
}

impl Handler for Server {
    fn on_open(&mut self, hs: Handshake) -> Result<()> {
        let jwt_token = hs.request.resource().split("=").collect::<Vec<_>>()[1];
        let jwt_struct = claim::decode_jwt(&jwt_token).unwrap();

        println!("{}", jwt_struct.user_id);
        println!("{}", self.master);

        self.user = jwt_struct.user_id;
        self.user_name = jwt_struct.username;

        let co = redis_co::get_redis_co();

        if self.user == self.master {
            create_console_message(&self.out, String::from("")).unwrap();
            create_log_message(
                &self.out,
                String::from(format!("give this id to your players: {}", self.game_id)),
                false,
            )
            .unwrap();

            redis_co::get_set_h_to_redis(
                self.game_id.to_string(),
                String::from("admin"),
                self.out.token(),
                self.out.connection_id(),
                co,
            );
        } else {
            redis_co::get_set_h_to_redis(
                self.game_id.to_string(),
                self.user.to_string(),
                self.out.token(),
                self.out.connection_id(),
                co,
            );
            let msg = format!("{{ \"user_id\": \"{}\", \"game_id\": \"{}\" }}", self.user.to_string(), self.game_id.to_string());

            send_owner_console_message(&self.out, self.game_id.to_string(), msg, ConsoleScope::PlayerConnection).unwrap();
        }

        // hs.request.header(header: &str)

        create_log_message(&self.out, format!("{} is connected", self.user_name), true).unwrap();

        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let raw_message = msg.into_text()?;

        println!("{}", raw_message);

        create_message(&self.out, raw_message, self.user_name.to_string(), true)
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        // The WebSocket protocol allows for a utf8 reason for the closing state after the
        // close code. WS-RS will attempt to interpret this data as a utf8 description of the
        // reason for closing the connection. I many cases, `reason` will be an empty string.
        // So, you may not normally want to display `reason` to the user,
        // but let's assume that we know that `reason` is human-readable.
        let _ = match code {
            CloseCode::Normal => {
                println!("The client is done with the connection.")
            }
            CloseCode::Away => println!("The client is leaving the site."),
            CloseCode::Abnormal => {
                println!("Closing handshake failed! Unable to obtain closing status from client.")
            }
            _ => println!("The client encountered an error: {}", reason),
        };
        let co = redis_co::get_redis_co();
        redis_co::remove_h_to_redis(co, self.user.to_string());

        create_log_message(
            &self.out,
            format!("{} is deconnected", self.user_name),
            true,
        )
        .unwrap();

        if self.user != self.master {
            let msg = format!("{{ \"user_id\": \"{}\", \"game_id\": \"{}\" }}", self.user.to_string(), self.game_id.to_string());
            send_owner_console_message(&self.out, self.game_id.to_string(), msg, ConsoleScope::PlayerLeave).unwrap();
        }
    }

    fn on_error(&mut self, err: Error) {
        println!("The server encountered an error: {:?}", err);
    }
}
