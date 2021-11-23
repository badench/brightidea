use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use warp::ws::{Message, WebSocket};

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;
pub type Rooms = Arc<RwLock<HashMap<String, Users>>>;

pub async fn join_room(ws: WebSocket, room_name: String, rooms: Rooms) {
    //Acquire write guard to get map of users and then update
    let mut room_guard = rooms.write().await;
    let users = match room_guard.get(&room_name).clone() {
        Some(users) => {
            users.clone()
        },
        None => {
            Users::default()
        }
    };
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);


    // Split the socket into a sender and receive of messages.
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    eprintln!("websocket send error: {}", e);
                })
                .await;
        }
    });

    // Save the sender in our list of connected users.
    users.write().await.insert(my_id, tx);
    room_guard.insert(room_name.clone(), users);
    drop(room_guard);

    // Return a `Future` that is basically a state machine managing
    // this specific user's connection.

    // Every time the user sends a message, broadcast it to
    // all other users...
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        user_message(my_id, msg, &room_name, &rooms).await;
    }

    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    user_disconnected(my_id, &room_name, &rooms).await;
}

async fn user_message(my_id: usize, msg: Message, room_name: &str, rooms: &Rooms) {
    // Skip any non-Text messages...
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };

    let new_msg = format!("<User#{}>: {}", my_id, msg);

    if let Some(users) = rooms.read().await.get(room_name) {
        // New message from this user, send it to everyone else (except same uid)...
        for (&uid, tx) in users.read().await.iter() {
            if my_id != uid {
                if let Err(_disconnected) = tx.send(Message::text(new_msg.clone())) {
                    // The tx is disconnected, our `user_disconnected` code
                    // should be happening in another task, nothing more to
                    // do here.
                }
            }
        }
    }
}

async fn user_disconnected(my_id: usize, room_name: &str, rooms: &Rooms) {
    eprintln!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    if let Some(users) = rooms.read().await.get(room_name) {
        users.write().await.remove(&my_id);
    }
}
