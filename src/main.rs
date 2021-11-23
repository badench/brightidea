// #![deny(warnings)]
use warp::Filter;

mod handlers;
use handlers::chat_handlers::Rooms;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    //Keep track of all rooms, key is a string representing a room, value
    // is a Users Map
    let rooms = Rooms::default();

    // Turn our "state" into a new Filter...
    let rooms = warp::any().map(move || rooms.clone());

    // GET /chat/<roomId> -> websocket upgrade
    let chat = warp::path!("chat" / String)
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(rooms)
        .map(|room_name: String, ws: warp::ws::Ws, rooms: Rooms| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| handlers::chat_handlers::join_room(socket, room_name, rooms))
        });

    // GET / -> index html
    let index = warp::path::end().map(|| {
        warp::reply::html(std::fs::read_to_string("src/www/index.html").unwrap())
    });

    let routes = index.or(chat);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}