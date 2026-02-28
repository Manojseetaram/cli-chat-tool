use mongodb::{bson::doc, options::FindOptions, Collection};
use serde_json::json;

use crate::types::{ChatMessage, Tx};

pub async fn send_history(mongo: &Collection<ChatMessage>, room: &str, tx: &Tx) {
    let opts = FindOptions::builder()
        .sort(doc! { "timestamp": 1 })
        .limit(100)
        .build();

    if let Ok(mut cursor) = mongo
        .find(doc! { "room": room, "deleted": false }, opts)
        .await
    {
        while cursor.advance().await.unwrap_or(false) {
            if let Ok(m) = cursor.deserialize_current() {
                let _ = tx.send(
                    json!({
                        "type":      "history",
                        "msg_id":    m.msg_id,
                        "nick":      m.nick,
                        "text":      m.text,
                        "timestamp": m.timestamp,
                        "edited":    m.edited,
                    })
                    .to_string(),
                );
            }
        }
    }
}

pub fn sv(val: &serde_json::Value, key: &str) -> String {
    val[key].as_str().unwrap_or("").to_string()
}