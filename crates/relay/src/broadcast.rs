use serde_json::json;

use crate::types::{Rooms, Tx};

pub async fn broadcast(rooms: &Rooms, room: &str, payload: &str) {
    let rooms = rooms.lock().await;
    if let Some(meta) = rooms.get(room) {
        for peer in &meta.peers {
            let _ = peer.tx.send(payload.to_string());
        }
    }
}

pub async fn broadcast_except(rooms: &Rooms, room: &str, payload: &str, skip_tx: &Tx) {
    let rooms = rooms.lock().await;
    if let Some(meta) = rooms.get(room) {
        for peer in &meta.peers {
            if peer.tx.same_channel(skip_tx) { continue; }
            let _ = peer.tx.send(payload.to_string());
        }
    }
}

pub async fn broadcast_system(rooms: &Rooms, room: &str, text: &str) {
    broadcast(
        rooms,
        room,
        &json!({ "type": "system", "text": text }).to_string(),
    )
    .await;
}