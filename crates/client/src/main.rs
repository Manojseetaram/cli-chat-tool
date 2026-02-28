use futures::{SinkExt, StreamExt};
use std::io::{self};
use terminal_size::{terminal_size, Width};
use tokio_tungstenite::connect_async;
use url::Url;

#[tokio::main]
async fn main() {
    let mut input = String::new();

    println!("Enter nickname:");
    io::stdin().read_line(&mut input).unwrap();
    let nick = input.trim().to_string();
    input.clear();

    println!("Enter secret key (room):");
    io::stdin().read_line(&mut input).unwrap();
    let room = input.trim().to_string();

    let url = Url::parse(&format!(
        "ws://127.0.0.1:3000/ws?room={}&nick={}",
        room, nick
    ))
    .unwrap();

    let (ws_stream, _) = connect_async(url).await.unwrap();
    println!("Connected. Type 'exit' to quit.");

    let (mut write, mut read) = ws_stream.split();
    let my_nick = nick.clone();

    // Receiving task
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            if let Ok(msg) = msg {
                if msg.is_text() {
                    let text = msg.into_text().unwrap();
                    let parts: Vec<&str> = text.split('|').collect();

                    if parts.len() == 2 {
                        print_message(parts[0], parts[1], &my_nick);
                    }
                }
            }
        }
    });

    loop {
        input.clear();
        io::stdin().read_line(&mut input).unwrap();
        let msg = input.trim();

        if msg == "exit" {
            println!("Do you want to exit? (y/N)");
            input.clear();
            io::stdin().read_line(&mut input).unwrap();
            if input.trim().to_lowercase() == "y" {
                println!("Exiting chat...");
                break;
            } else {
                continue;
            }
        }

        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                msg.to_string(),
            ))
            .await
            .unwrap();
    }
}

fn print_message(sender: &str, message: &str, my_nick: &str) {
    let width = if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    };

    let formatted = format!("{}: {}", sender, message);

    if sender == my_nick {
        // RIGHT SIDE (Your message)
        let padding = width.saturating_sub(formatted.len());
        println!("{}{}", " ".repeat(padding), formatted);
    } else {
        // LEFT SIDE (Friend message)
        println!("{}", formatted);
    }
}