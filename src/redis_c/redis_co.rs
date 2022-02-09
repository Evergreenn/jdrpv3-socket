use redis::Connection;
use serde_json;
use ws::util::Token;

pub fn get_redis_co() -> Connection {
    let client = redis::Client::open(dotenv!("REDIS_DSN")).unwrap();
    client.get_connection().unwrap()
}

pub fn register_address_to_redis(game_id: String, ws_addr: &String, mut co: Connection) {
    
    let _: () = redis::cmd("HSET")
        .arg(game_id)
        .arg("ws_address")
        .arg(ws_addr)
        .query(&mut co)
        .unwrap();
}

pub fn get_set_h_to_redis(
    game_id: String,
    player_id: String,
    playersocket_tokenid: Token,
    playerconnection_id: u32,
    mut co: Connection,
) {
    let info = vec![
        usize::from(playersocket_tokenid),
        playerconnection_id as usize,
    ];
    let to_store = serde_json::to_string(&info).unwrap();

    let _: () = redis::cmd("HSET")
        .arg(game_id)
        .arg(player_id)
        .arg(to_store)
        .query(&mut co)
        .unwrap();

    // let mut vec: crate::Room = serde_json::from_str(&room_from_redis).unwrap();

    // vec.items.push(user_id);

    // println!("Vector from room: {:#?}", vec);
    // let to_store = serde_json::to_string(&vec).unwrap();
    // let _: () = redis::cmd("HSET")
    //     .arg("ROOM")
    //     .arg("people")
    //     .arg(to_store)
    //     .query(&mut co)
    //     .unwrap();
}

pub fn get_token_connectionid(
    game_id: String,
    player_id: String,
    mut co: Connection,
) -> (Token, u32) {
    let id: String = redis::cmd("HGET")
        .arg(game_id)
        .arg(player_id)
        .query(&mut co)
        .unwrap();

    let rslt: Vec<u8> = serde_json::from_str(&id).unwrap();

    (Token::from(rslt[0] as usize), u32::from(rslt[1]))
}

//TODO: REWORK THIS ; should take a game id. ROOM doesn't exist anymore.
pub fn remove_h_to_redis(mut co: Connection, user_id: String) {
    let room_from_redis: String = redis::cmd("HGET")
        .arg("ROOM")
        .arg("people")
        .query(&mut co)
        .unwrap();

    let mut vec: crate::Room = serde_json::from_str(&room_from_redis).unwrap();

    if let Some(pos) = vec.items.iter().position(|x| *x == user_id) {
        vec.items.remove(pos);
    }

    let to_store = serde_json::to_string(&vec).unwrap();
    let _: () = redis::cmd("HSET")
        .arg("ROOM")
        .arg("people")
        .arg(to_store)
        .query(&mut co)
        .unwrap();
}
