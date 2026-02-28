use axum::{routing::post, Router};
use std::net::SocketAddr;

mod routes;
mod storage;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/send", post(routes::send_message))
        .route("/fetch/:room_id", post(routes::fetch_messages));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    println!("Relay running on http://{}", addr);


    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app)
        .await
        .expect("Server crashed");
}