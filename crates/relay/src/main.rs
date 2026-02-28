mod broadcast;
mod db;
mod events;
mod socket;
mod types;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

use axum::{routing::get, Router};
use mongodb::{options::ClientOptions, Client as MongoClient, Collection};

use socket::ws_handler;
use types::{AppState, ChatMessage};

#[tokio::main]
async fn main() {
    let mongo_uri = std::env::var("MONGO_URI")
        .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());

    let opts = ClientOptions::parse(&mongo_uri).await.expect("bad MONGO_URI");
    let mongo_client = MongoClient::with_options(opts).expect("mongo connect failed");
    let collection: Collection<ChatMessage> =
        mongo_client.database("viva_chat").collection("messages");

    println!("✓ MongoDB connected");

    let state = AppState {
        rooms: Arc::new(Mutex::new(HashMap::new())),
        mongo: collection,
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3002);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("✓ Relay listening on ws://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind failed");
    axum::serve(listener, app).await.expect("server crashed");
}