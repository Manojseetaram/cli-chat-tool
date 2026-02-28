use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc;

use crate::{
    broadcast::broadcast_system,
    db::send_history,
    events::{drain_and_close, handle_event},
    types::{AppState, RoomMeta, RoomPeer, WsParams},
};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(p): Query<WsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, p, state))
}

fn room_id(room_key: &str, nick: &str, friend: &str) -> String {
    let mut pair = [nick.to_lowercase(), friend.to_lowercase()];
    pair.sort();
    format!("{}::{}::{}", room_key.trim(), pair[0], pair[1])
}

async fn handle_socket(socket: WebSocket, params: WsParams, state: AppState) {
    let (mut ws_send, mut ws_recv) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let nick     = params.nick.trim().to_string();
    let friend   = params.friend.trim().to_string();
    let raw_room = params.room.trim().to_string();

    // ── Validate inputs ───────────────────────────────────────────────────────
    if nick.is_empty() || friend.is_empty() || raw_room.is_empty() {
        let _ = tx.send(
            json!({ "type": "error", "text": "nick, friend, and room key are all required." })
                .to_string(),
        );
        drain_and_close(ws_send, rx).await;
        return;
    }

    if nick == friend {
        let _ = tx.send(
            json!({ "type": "error", "text": "Your nickname and your friend's nickname must be different." })
                .to_string(),
        );
        drain_and_close(ws_send, rx).await;
        return;
    }

    let room = room_id(&raw_room, &nick, &friend);

    // ── Room access control ───────────────────────────────────────────────────
    {
        let mut rooms = state.rooms.lock().await;
        let meta = rooms.entry(room.clone()).or_insert_with(|| RoomMeta {
            allowed: {
                let mut pair = [nick.clone(), friend.clone()];
                pair.sort();
                pair
            },
            peers: Vec::new(),
        });

        if !meta.allowed.contains(&nick) || !meta.allowed.contains(&friend) {
            let _ = tx.send(
                json!({
                    "type": "error",
                    "text": "Access denied: nickname or friend key does not match this room."
                })
                .to_string(),
            );
            drop(rooms);
            drain_and_close(ws_send, rx).await;
            return;
        }

        // Prune stale/dead peers with the same nick.
        meta.peers.retain(|p| !(p.nick == nick && p.tx.is_closed()));

        if meta.peers.iter().any(|p| p.nick == nick) {
            let _ = tx.send(
                json!({
                    "type": "error",
                    "text": "This nickname is already connected in this room."
                })
                .to_string(),
            );
            drop(rooms);
            drain_and_close(ws_send, rx).await;
            return;
        }

        meta.peers.push(RoomPeer { nick: nick.clone(), tx: tx.clone() });
    }

    send_history(&state.mongo, &room, &tx).await;
    broadcast_system(&state.rooms, &room, &format!("{} joined", nick)).await;

    // ── Forward outbound messages to the WebSocket ────────────────────────────
    let fwd = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_send.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // ── Receive loop ──────────────────────────────────────────────────────────
    while let Some(Ok(msg)) = ws_recv.next().await {
        if let Message::Text(raw) = msg {
            if let Ok(val) = serde_json::from_str(&raw) {
                handle_event(&val, &nick, &room, &tx, &state).await;
            }
        }
    }

    // ── Cleanup on disconnect ─────────────────────────────────────────────────
    {
        let mut rooms = state.rooms.lock().await;
        if let Some(meta) = rooms.get_mut(&room) {
            meta.peers.retain(|p| p.nick != nick);
            if meta.peers.is_empty() {
                rooms.remove(&room);
            }
        }
    }

    fwd.abort();
    broadcast_system(&state.rooms, &room, &format!("{} left", nick)).await;
}