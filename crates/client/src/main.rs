mod crypto;
mod network;
use sha2::Digest;
use clap::{Parser, Subcommand};
use uuid::Uuid;
use chrono::Utc;
use common::models::Message;
use std::io::{self, Write};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Chat {
        #[arg(long)]
        key: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Chat { key } => {
            let room_id = format!("{:x}", sha2::Sha256::digest(key.as_bytes()));

            println!("Enter message:");
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            let encrypted = crypto::encrypt(&key, input.trim());

            let message = Message {
                id: Uuid::new_v4().to_string(),
                room_id: room_id.clone(),
                sender: "Manoj".into(),
                timestamp: Utc::now().timestamp(),
                encrypted_payload: encrypted,
            };

            network::send(&message).await;

            println!("Message sent!");

            let messages = network::fetch(&room_id).await;

            for msg in messages {
                let text = crypto::decrypt(&key, &msg.encrypted_payload);
                println!("{}: {}", msg.sender, text);
            }
        }
    }
}