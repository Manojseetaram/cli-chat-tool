use mongodb::Collection;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex};

pub type Tx = mpsc::UnboundedSender<String>;

pub struct RoomPeer {
    pub nick: String,
    pub tx:   Tx,
}

pub struct RoomMeta {
    pub allowed: [String; 2],
    pub peers:   Vec<RoomPeer>,
}

pub type Rooms = Arc<Mutex<HashMap<String, RoomMeta>>>;

#[derive(Clone)]
pub struct AppState {
    pub rooms: Rooms,
    pub mongo: Collection<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub msg_id:    String,
    pub room:      String,
    pub nick:      String,
    pub text:      String,
    pub timestamp: i64,
    pub deleted:   bool,
    pub edited:    bool,
}

#[derive(Deserialize)]
pub struct WsParams {
    pub room:   String,
    pub nick:   String,
    pub friend: String,
}