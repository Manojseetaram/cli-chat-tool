use axum::{Json, extract::Path};
use common::models::Message;

pub async fn send_message(Json(_msg): Json<Message>) -> &'static str {
    "Message received"
}

pub async fn fetch_messages(Path(_room_id): Path<String>) -> Json<Vec<Message>> {
    Json(vec![])
}