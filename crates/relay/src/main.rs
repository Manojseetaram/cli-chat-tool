use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};
use tokio::sync::{Mutex, mpsc};

type Tx = mpsc::UnboundedSender<Message>;
type Rooms = Arc<Mutex<HashMap<String, Vec<Tx>>>>;

#[derive(Deserialize)]
struct WsParams {
    room: String,
    nick: String,
}

#[tokio::main]
async fn main() {
    let rooms: Rooms = Arc::new(Mutex::new(HashMap::new()));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(rooms);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    println!("Relay running at ws://{}", addr);

    // NEW Axum 0.7 server API
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app)
        .await
        .expect("Server crashed");
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(rooms): State<Rooms>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, params, rooms))
}

async fn handle_socket(
    socket: WebSocket,
    params: WsParams,
    rooms: Rooms,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    {
        let mut rooms_lock = rooms.lock().await;
        rooms_lock
            .entry(params.room.clone())
            .or_default()
            .push(tx);
    }

    let room = params.room.clone();
    let nick = params.nick.clone();

    // Task 1: send messages to this client
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = sender.send(msg).await;
        }
    });

    // Task 2: receive messages from this client and broadcast to room
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            let broadcast = format!("{}|{}", nick, text);

            let rooms_lock = rooms.lock().await;
            if let Some(clients) = rooms_lock.get(&room) {
                for client in clients {
                    let _ = client.send(Message::Text(broadcast.clone()));
                }
            }
        }
    }
}