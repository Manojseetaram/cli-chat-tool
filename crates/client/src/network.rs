// use common::models::Message;
// use reqwest::Client;

// const RELAY_URL: &str = "http://127.0.0.1:3000";

// pub async fn send(msg: &Message) {
//     let client = Client::new();
//     client.post(format!("{}/send", RELAY_URL))
//         .json(msg)
//         .send()
//         .await
//         .unwrap();
// }

// pub async fn fetch(room_id: &str) -> Vec<Message> {
//     let client = Client::new();
//     let res = client.post(format!("{}/fetch/{}", RELAY_URL, room_id))
//         .send()
//         .await
//         .unwrap();

//     res.json().await.unwrap()
// }