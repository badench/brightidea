// #![deny(warnings)]
use warp::Filter;
use std::sync::Arc;

mod handlers;

use handlers::chat_handlers::Rooms;

mod logger;

use logger::Logger;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    //Keep track of all rooms, key is a string representing a room, value
    // is a Users Map
    let rooms = Rooms::default();

    //Create a logger
    let logger = Arc::new(Logger::new());

    let rooms = warp::any().map(move || rooms.clone());
    let logger = warp::any().map(move || logger.clone());

    // GET /chat/<roomId> -> websocket upgrade
    let chat = warp::path!("chat" / String)
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(rooms)
        .and(logger)
        .map(|room_name: String, ws: warp::ws::Ws, rooms: Rooms, logger: Arc<Logger>| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| handlers::chat_handlers::join_room(socket, room_name, rooms, logger.clone()))
        });

    // GET / -> index html
    let index = warp::path::end().map(|| {
        warp::reply::html(std::fs::read_to_string("src/www/index.html").unwrap())
    });

    let routes = index.or(chat);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}