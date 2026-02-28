use axum::{Json, extract::Path};
use common::models::Message;
use crate::storage::MESSAGE_STORE;

pub async fn send_message(Json(msg): Json<Message>) -> &'static str {
    let mut store = MESSAGE_STORE.lock().unwrap();
    store.entry(msg.room_id.clone())
        .or_default()
        .push(msg);
    "Stored"
}

pub async fn fetch_messages(Path(room_id): Path<String>) -> Json<Vec<Message>> {
    let mut store = MESSAGE_STORE.lock().unwrap();
 let messages = store.get(&room_id).cloned().unwrap_or_default();
    Json(messages)
}