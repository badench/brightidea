// #![deny(warnings)]
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize},
    Arc,
};

use tokio::sync::{mpsc, RwLock};
use warp::ws::{Message};
use warp::Filter;

mod handlers;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

//TODO:
// 1. Input room Id
// 2. click "Enter Room" which changes messages from "Connecting" to "Connected to room#" and establishes websocket
// 3. adjust route for message sending to be /chat/room
// 4. Create struct to model 'room'

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Keep track of all connected users, key is usize, value
    // is a websocket sender.
    let users = Users::default();
    // Turn our "state" into a new Filter...
    let users = warp::any().map(move || users.clone());

    // GET /chat -> websocket upgrade
    let chat = warp::path!("chat" / String)
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(users)
        .map(|room_name, ws: warp::ws::Ws, users| {
            // This will call our function if the handshake succeeds.
            eprintln!("room Id {}", room_name);
            ws.on_upgrade(move |socket| handlers::chat_handlers::user_connected(socket, users))
        });

    // GET / -> index html
    let index = warp::path::end().map(|| {
        warp::reply::html(std::fs::read_to_string("src/www/index.html").unwrap())
    });

    let routes = index.or(chat);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}