use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use mongodb::{bson::doc, options::{ClientOptions, FindOptions}, Client as MongoClient, Collection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

type Tx = mpsc::UnboundedSender<String>;

struct RoomPeer { nick: String, tx: Tx }

struct RoomMeta {
    peers: Vec<RoomPeer>,
    allowed_nicks: Vec<String>,
}

type Rooms = Arc<Mutex<HashMap<String, RoomMeta>>>;

#[derive(Clone)]
struct AppState { rooms: Rooms, mongo: Collection<ChatMessage> }

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    msg_id: String, room: String, nick: String, text: String,
    timestamp: i64, deleted: bool, edited: bool,
}

#[derive(Deserialize)]
struct WsParams { room: String, nick: String, friend: String }

#[tokio::main]
async fn main() {
    let mongo_uri = std::env::var("MONGO_URI")
        .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
    let opts = ClientOptions::parse(&mongo_uri).await.expect("bad MONGO_URI");
    let mongo_client = MongoClient::with_options(opts).expect("mongo connect failed");
    let collection: Collection<ChatMessage> = mongo_client.database("viva_chat").collection("messages");
    println!("✓ MongoDB connected");

    let state = AppState { rooms: Arc::new(Mutex::new(HashMap::new())), mongo: collection };
    let app = Router::new().route("/ws", get(ws_handler)).with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("✓ Relay listening on ws://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind failed");
    axum::serve(listener, app).await.expect("server crashed");
}

async fn ws_handler(ws: WebSocketUpgrade, Query(p): Query<WsParams>, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, p, state))
}

async fn handle_socket(socket: WebSocket, params: WsParams, state: AppState) {
    let (mut ws_send, mut ws_recv) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let nick   = params.nick.trim().to_string();
    let friend = params.friend.trim().to_string();
    let room   = params.room.trim().to_string();

    // Auth
    {
        let mut rooms = state.rooms.lock().await;
        let meta = rooms.entry(room.clone()).or_insert_with(|| RoomMeta {
            peers: Vec::new(),
            allowed_nicks: vec![nick.clone(), friend.clone()],
        });
        let nick_ok   = meta.allowed_nicks.contains(&nick);
        let friend_ok = meta.allowed_nicks.contains(&friend);
        if !nick_ok || !friend_ok {
            let _ = tx.send(json!({ "type": "error", "text": "Access denied: nickname or friend key does not match this room." }).to_string());
            drop(rooms);
            tokio::spawn(async move {
                if let Some(msg) = rx.recv().await { let _ = ws_send.send(Message::Text(msg)).await; }
            });
            return;
        }
        meta.peers.push(RoomPeer { nick: nick.clone(), tx: tx.clone() });
    }

    send_history(&state.mongo, &room, &tx).await;
    broadcast_system(&state.rooms, &room, &format!("── {} joined ──", nick)).await;

    let fwd = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_send.send(Message::Text(msg)).await.is_err() { break; }
        }
    });

    while let Some(Ok(msg)) = ws_recv.next().await {
        if let Message::Text(raw) = msg {
            if let Ok(val) = serde_json::from_str::<Value>(&raw) {
                handle_event(&val, &nick, &room, &tx, &state).await;
            }
        }
    }

    { let mut rooms = state.rooms.lock().await; if let Some(meta) = rooms.get_mut(&room) { meta.peers.retain(|p| !p.tx.is_closed()); } }
    fwd.abort();
    broadcast_system(&state.rooms, &room, &format!("── {} left ──", nick)).await;
}

async fn handle_event(val: &Value, nick: &str, room: &str, sender_tx: &Tx, state: &AppState) {
    match val["type"].as_str().unwrap_or("") {
        "msg" => {
            let text = match val["text"].as_str() { Some(t) if !t.trim().is_empty() => t.to_string(), _ => return };
            let msg_id = Uuid::new_v4().to_string();
            let ts = Utc::now().timestamp_millis();
            let _ = state.mongo.insert_one(&ChatMessage { msg_id: msg_id.clone(), room: room.to_string(), nick: nick.to_string(), text: text.clone(), timestamp: ts, deleted: false, edited: false }, None).await;
            let payload = json!({ "type": "msg", "msg_id": msg_id, "nick": nick, "text": text, "timestamp": ts, "edited": false }).to_string();
            // Broadcast to others only
            broadcast_except(&state.rooms, room, &payload, sender_tx).await;
            // Send ack to sender
            let _ = sender_tx.send(json!({ "type": "ack", "msg_id": msg_id, "timestamp": ts }).to_string());
        }
        "edit" => {
            let msg_id = sv(val, "msg_id"); let new_text = sv(val, "text");
            if msg_id.is_empty() || new_text.trim().is_empty() { return; }
            let _ = state.mongo.update_one(doc! { "msg_id": &msg_id, "nick": nick, "room": room }, doc! { "$set": { "text": &new_text, "edited": true } }, None).await;
            broadcast(&state.rooms, room, &json!({ "type": "edit", "msg_id": msg_id, "text": new_text }).to_string()).await;
        }
        "delete" => {
            let msg_id = sv(val, "msg_id"); if msg_id.is_empty() { return; }
            let _ = state.mongo.update_one(doc! { "msg_id": &msg_id, "nick": nick, "room": room }, doc! { "$set": { "deleted": true } }, None).await;
            broadcast(&state.rooms, room, &json!({ "type": "delete", "msg_id": msg_id }).to_string()).await;
        }
        _ => {}
    }
}

async fn send_history(mongo: &Collection<ChatMessage>, room: &str, tx: &Tx) {
    let opts = FindOptions::builder().sort(doc! { "timestamp": 1 }).limit(100).build();
    if let Ok(mut cursor) = mongo.find(doc! { "room": room, "deleted": false }, opts).await {
        while cursor.advance().await.unwrap_or(false) {
            if let Ok(m) = cursor.deserialize_current() {
                let _ = tx.send(json!({ "type": "history", "msg_id": m.msg_id, "nick": m.nick, "text": m.text, "timestamp": m.timestamp, "edited": m.edited }).to_string());
            }
        }
    }
}

async fn broadcast(rooms: &Rooms, room: &str, payload: &str) {
    let rooms = rooms.lock().await;
    if let Some(meta) = rooms.get(room) { for peer in &meta.peers { let _ = peer.tx.send(payload.to_string()); } }
}

async fn broadcast_except(rooms: &Rooms, room: &str, payload: &str, skip_tx: &Tx) {
    let rooms = rooms.lock().await;
    if let Some(meta) = rooms.get(room) {
        for peer in &meta.peers {
            if peer.tx.same_channel(skip_tx) { continue; }
            let _ = peer.tx.send(payload.to_string());
        }
    }
}

async fn broadcast_system(rooms: &Rooms, room: &str, text: &str) {
    broadcast(rooms, room, &json!({ "type": "system", "text": text }).to_string()).await;
}

fn sv(val: &Value, key: &str) -> String { val[key].as_str().unwrap_or("").to_string() }