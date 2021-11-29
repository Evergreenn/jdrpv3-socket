use redis::Connection;

pub fn get_redis_co() -> Connection {
    let client = redis::Client::open(dotenv!("REDIS_DSN")).unwrap();
    client.get_connection().unwrap()
}

pub fn get_set_h_to_redis(mut co: Connection, user_id: String) -> () {
    let room_from_redis: String = redis::cmd("HGET")
        .arg("ROOM")
        .arg("people")
        .query(&mut co)
        .unwrap();

    let mut vec: crate::Room = serde_json::from_str(&room_from_redis).unwrap();

    vec.items.push(user_id);

    println!("Vector from room: {:#?}", vec);
    let to_store = serde_json::to_string(&vec).unwrap();
    let _: () = redis::cmd("HSET")
        .arg("ROOM")
        .arg("people")
        .arg(to_store)
        .query(&mut co)
        .unwrap();
}

pub fn remove_h_to_redis(mut co: Connection, user_id: String) -> () {
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
