mod cli;
mod crypto;
mod network;
mod storage;
mod sync;
use clap::{Parser, Subcommand};

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
            println!("Starting chat with key: {}", key);
        }
    }
}