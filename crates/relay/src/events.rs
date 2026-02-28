use axum::extract::ws::Message;
use chrono::Utc;
use futures::SinkExt;
use mongodb::bson::doc;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    broadcast::{broadcast, broadcast_except},
    db::sv,
    types::{AppState, ChatMessage, Tx},
};

pub async fn handle_event(
    val: &Value,
    nick: &str,
    room: &str,
    sender_tx: &Tx,
    state: &AppState,
) {
    match val["type"].as_str().unwrap_or("") {
        "msg" => {
            let text = match val["text"].as_str() {
                Some(t) if !t.trim().is_empty() => t.to_string(),
                _ => return,
            };

            let msg_id = Uuid::new_v4().to_string();
            let ts = Utc::now().timestamp_millis();

            let _ = state
                .mongo
                .insert_one(
                    &ChatMessage {
                        msg_id:    msg_id.clone(),
                        room:      room.to_string(),
                        nick:      nick.to_string(),
                        text:      text.clone(),
                        timestamp: ts,
                        deleted:   false,
                        edited:    false,
                    },
                    None,
                )
                .await;

            let payload = json!({
                "type":      "msg",
                "msg_id":    msg_id,
                "nick":      nick,
                "text":      text,
                "timestamp": ts,
                "edited":    false,
            })
            .to_string();

            broadcast_except(&state.rooms, room, &payload, sender_tx).await;

            let _ = sender_tx.send(
                json!({
                    "type":      "ack",
                    "msg_id":    msg_id,
                    "timestamp": ts,
                })
                .to_string(),
            );
        }

        "edit" => {
            let msg_id   = sv(val, "msg_id");
            let new_text = sv(val, "text");
            if msg_id.is_empty() || new_text.trim().is_empty() { return; }

            let res = state
                .mongo
                .update_one(
                    doc! { "msg_id": &msg_id, "nick": nick, "room": room },
                    doc! { "$set": { "text": &new_text, "edited": true } },
                    None,
                )
                .await;

            if let Ok(r) = res {
                if r.matched_count == 0 {
                    let _ = sender_tx.send(
                        json!({
                            "type": "error",
                            "text": "Message not found or not yours to edit."
                        })
                        .to_string(),
                    );
                    return;
                }
            }

            broadcast(
                &state.rooms,
                room,
                &json!({
                    "type":   "edit",
                    "msg_id": msg_id,
                    "text":   new_text,
                })
                .to_string(),
            )
            .await;
        }

        "delete" => {
            let msg_id = sv(val, "msg_id");
            if msg_id.is_empty() { return; }

            let res = state
                .mongo
                .update_one(
                    doc! { "msg_id": &msg_id, "nick": nick, "room": room },
                    doc! { "$set": { "deleted": true } },
                    None,
                )
                .await;

            if let Ok(r) = res {
                if r.matched_count == 0 {
                    let _ = sender_tx.send(
                        json!({
                            "type": "error",
                            "text": "Message not found or not yours to delete."
                        })
                        .to_string(),
                    );
                    return;
                }
            }

            broadcast(
                &state.rooms,
                room,
                &json!({
                    "type":   "delete",
                    "msg_id": msg_id,
                })
                .to_string(),
            )
            .await;
        }

        _ => {}
    }
}

pub async fn drain_and_close<S>(mut ws_send: S, mut rx: mpsc::UnboundedReceiver<String>)
where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    while let Ok(msg) = rx.try_recv() {
        let _ = ws_send.send(Message::Text(msg)).await;
    }
    let _ = ws_send.close().await;
}