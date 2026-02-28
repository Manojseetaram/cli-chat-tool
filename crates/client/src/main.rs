mod colors;
mod types;
mod utils;
mod ui;
mod chat;

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    collections::HashMap,
};

use serde_json::json;
use futures::{SinkExt, StreamExt};
use url::Url;

use colors::*;
use types::{Msg, Store, Order, Pending};
use utils::{ask, clear, enc, ns};
use ui::{banner, bubble, err, help, history};
use chat::{handle_in, mine, send, upsert};

const DEFAULT_RELAY: &str = "cli-chat-tool-g1n0.onrender.com";

#[tokio::main]
async fn main() {
    clear();
    banner();

    println!("  {DG}All three fields must match on both sides to enter the room.{R}\n");

    let nick = loop {
        let n = ask("  Your nickname     › ");
        if n.is_empty() { println!("  {RE}⚠  Nickname cannot be empty.{R}"); continue; }
        if n.len() > 32  { println!("  {RE}⚠  Nickname too long (max 32 chars).{R}"); continue; }
        break n;
    };
    let friend = loop {
        let f = ask("  Friend's nickname  › ");
        if f.is_empty() { println!("  {RE}⚠  Friend's nickname cannot be empty.{R}"); continue; }
        if f == nick    { println!("  {RE}⚠  Your nickname and friend's nickname must be different.{R}"); continue; }
        if f.len() > 32 { println!("  {RE}⚠  Nickname too long (max 32 chars).{R}"); continue; }
        break f;
    };
    let room = loop {
        let r = ask("  Secret room key   › ");
        if r.is_empty() { println!("  {RE}⚠  Room key cannot be empty.{R}"); continue; }
        break r;
    };

    let relay  = std::env::var("RELAY").unwrap_or_else(|_| DEFAULT_RELAY.to_string());
    let scheme = if relay.contains("railway.app") || relay.contains("render.com")
        || relay.contains("fly.dev") || relay.contains("onrender.com") { "wss" } else { "ws" };

    let url = Url::parse(&format!(
        "{}://{}/ws?room={}&nick={}&friend={}",
        scheme, relay, enc(&room), enc(&nick), enc(&friend)
    )).unwrap();

    print!("\n  {DG}Connecting to {}...{R}", relay);
    io::stdout().flush().unwrap();

    let (ws_stream, _) = match tokio_tungstenite::connect_async(url.as_str()).await {
        Ok(r)  => r,
        Err(e) => {
            println!("\r  {RE}✗  Could not connect: {e}{R}");
            println!("  {DG}  Try: RELAY=your-server.onrender.com viva{R}\n");
            std::process::exit(1);
        }
    };

    let tw = utils::twidth();
    println!("\r{DG}{}{R}", "─".repeat(tw));
    println!("  {DG}room:{R} {room}   {DG}you:{R} {GR}{nick}{R}   {DG}friend:{R} {Y}{friend}{R}   {DG}relay:{R} {relay}");
    println!("{DG}{}{R}", "─".repeat(tw));
    println!("  {DG}Type and Enter to send  ·  /help for commands{R}");
    println!("{DG}{}{R}\n", "─".repeat(tw));

    let (mut ws_w, mut ws_r) = ws_stream.split();
    let store:   Store   = Arc::new(Mutex::new(HashMap::new()));
    let order:   Order   = Arc::new(Mutex::new(Vec::new()));
    let pending: Pending = Arc::new(Mutex::new(Vec::new()));

    let (exit_tx, mut exit_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let exit_tx2 = exit_tx.clone();
    ctrlc::set_handler(move || { let _ = exit_tx2.send(()); }).ok();

    let (s2, o2, p2, n2, f2) = (
        store.clone(), order.clone(), pending.clone(),
        nick.clone(), friend.clone(),
    );

    let typing: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let typing_recv  = typing.clone();
    let typing_stdin = typing.clone();

    // ── Receive task ──────────────────────────────────────────────────────────
    tokio::spawn(async move {
        while let Some(Ok(raw)) = ws_r.next().await {
            if let Ok(text) = raw.into_text() {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if val["type"].as_str() == Some("error") {
                        let msg = val["text"].as_str().unwrap_or("Unknown error").to_string();
                        let partial = typing_recv.lock().unwrap().clone();
                        print!("\r\x1b[K");
                        println!("\n  {RE}✗  {msg}{R}");
                        println!("  {DG}  Make sure both sides use the exact same nick, friend, and room key.{R}\n");
                        print!("{DG}  ›{R} {partial}");
                        io::stdout().flush().unwrap();
                        std::process::exit(1);
                    }

                    let partial = typing_recv.lock().unwrap().clone();
                    print!("\r\x1b[K");
                    io::stdout().flush().unwrap();
                    handle_in(&val, &n2, &f2, &s2, &o2, &p2);
                    print!("{DG}  ›{R} {partial}");
                    io::stdout().flush().unwrap();
                }
            }
        }
        print!("\r\x1b[K");
        println!("\n  {RE}✗  Disconnected from server.{R}\n");
        io::stdout().flush().unwrap();
        std::process::exit(1);
    });

    // ── Stdin reader ──────────────────────────────────────────────────────────
    let (line_tx, mut line_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    std::thread::spawn(move || {
        loop {
            let mut buf = String::new();
            match io::stdin().read_line(&mut buf) {
                Ok(0) | Err(_) => { let _ = line_tx.send("\x04".to_string()); break; }
                Ok(_) => {
                    *typing_stdin.lock().unwrap() = String::new();
                    let _ = line_tx.send(buf.trim().to_string());
                }
            }
        }
    });

    print!("{DG}  ›{R} ");
    io::stdout().flush().unwrap();

    loop {
        tokio::select! {
            _ = exit_rx.recv() => {
                print!("\r\x1b[K");
                println!("\n  {CY}Thank you, bye! 👋{R}\n");
                io::stdout().flush().unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                std::process::exit(0);
            }

            maybe = line_rx.recv() => {
                let line = match maybe { Some(l) => l, None => break };

                if line == "\x04" {
                    print!("\r\x1b[K");
                    println!("\n  {CY}Thank you, bye! 👋{R}\n");
                    io::stdout().flush().unwrap();
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    std::process::exit(0);
                }

                if line.is_empty() {
                    print!("{DG}  ›{R} ");
                    io::stdout().flush().unwrap();
                    continue;
                }

                print!("\x1b[1A\r\x1b[K");
                io::stdout().flush().unwrap();

                match line.as_str() {
                    "exit" | "/exit" | "/quit" | "bye" | "quit" => {
                        println!("\n  {CY}Thank you, bye! 👋{R}\n");
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                        std::process::exit(0);
                    }
                    "/help"    => help(),
                    "/history" => history(&store, &order, &nick),
                    _ if line.starts_with("/edit ") => {
                        let rest = &line[6..];
                        let mut p = rest.splitn(2, ' ');
                        let n = p.next().unwrap_or("").trim();
                        let t = p.next().unwrap_or("").trim();
                        if t.is_empty() {
                            err("Usage: /edit <N> <new text>");
                        } else {
                            match mine(n, &order, &store, &nick) {
                                Some(id) => send(&mut ws_w, json!({"type":"edit","msg_id":id,"text":t})).await,
                                None     => err("Message not found or not yours"),
                            }
                        }
                    }
                    _ if line.starts_with("/delete ") => {
                        match mine(line[8..].trim(), &order, &store, &nick) {
                            Some(id) => send(&mut ws_w, json!({"type":"delete","msg_id":id})).await,
                            None     => err("Message not found or not yours"),
                        }
                    }
                    _ if line.starts_with('/') => err("Unknown command — /help"),
                    _ => {
                        let lid = format!("p-{}", ns());
                        let lts = chrono::Utc::now().timestamp_millis();
                        let idx = upsert(&store, &order, Msg {
                            msg_id: lid.clone(),
                            nick:   nick.clone(),
                            text:   line.clone(),
                            ts:     lts,
                            edited: false,
                        });
                        pending.lock().unwrap().push(lid);
                        bubble(idx, &nick, &line, lts, false, &nick);
                        send(&mut ws_w, json!({"type":"msg","text":line})).await;
                    }
                }

                print!("{DG}  ›{R} ");
                io::stdout().flush().unwrap();
            }
        }
    }
}