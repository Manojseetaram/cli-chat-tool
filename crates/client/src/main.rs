use chrono::{Local, TimeZone};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{Arc, Mutex},
};
use terminal_size::{terminal_size, Width};
use tokio_tungstenite::connect_async;
use url::Url;

// ── ANSI colors ───────────────────────────────────────────────────────────────
const R:  &str = "\x1b[0m";
const W:  &str = "\x1b[97m";   // white  — your messages
const Y:  &str = "\x1b[93m";   // yellow — friend messages
const DG: &str = "\x1b[90m";   // dark gray — borders, meta
const CY: &str = "\x1b[96m";   // cyan — system / banner
const RE: &str = "\x1b[91m";   // red — errors
const GR: &str = "\x1b[92m";   // green — success

// ── State ─────────────────────────────────────────────────────────────────────
#[derive(Clone)]
struct Msg {
    msg_id: String,
    nick:   String,
    text:   String,
    ts:     i64,
    edited: bool,
}

type Store   = Arc<Mutex<HashMap<String, Msg>>>;
type Order   = Arc<Mutex<Vec<String>>>;
type Pending = Arc<Mutex<Vec<String>>>;

// Shared flag so the receive task can signal a fatal error (auth failure)
type ErrFlag = Arc<Mutex<Option<String>>>;

// ── Main ──────────────────────────────────────────────────────────────────────
#[tokio::main]
async fn main() {
    clear();
    banner();

    // ── Collect credentials ───────────────────────────────────────────────────
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

    // ── Connect ───────────────────────────────────────────────────────────────
    let relay  = std::env::var("RELAY").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let scheme = if relay.contains("railway.app")
        || relay.contains("render.com")
        || relay.contains("fly.dev")
        || relay.contains("onrender.com")
    { "wss" } else { "ws" };

    let url = Url::parse(&format!(
        "{}://{}/ws?room={}&nick={}&friend={}",
        scheme, relay, enc(&room), enc(&nick), enc(&friend)
    )).unwrap();

    print!("\n  {DG}Connecting to {}...{R}", relay);
    io::stdout().flush().unwrap();

    let (ws_stream, _) = match connect_async(url.as_str()).await {
        Ok(r)  => r,
        Err(e) => {
            println!("\r  {RE}✗  Could not connect: {e}{R}");
            println!("  {DG}  Make sure RELAY is set correctly, e.g.:{R}");
            println!("  {DG}  RELAY=your-server.onrender.com viva{R}\n");
            std::process::exit(1);
        }
    };

    let tw = twidth();
    println!("\r{DG}{}{R}", "─".repeat(tw));
    println!("  {DG}room:{R} {room}   {DG}you:{R} {GR}{nick}{R}   {DG}friend:{R} {Y}{friend}{R}   {DG}relay:{R} {relay}");
    println!("{DG}{}{R}", "─".repeat(tw));
    println!("  {DG}Type and Enter to send  ·  /help for commands{R}");
    println!("{DG}{}{R}\n", "─".repeat(tw));

    let (mut ws_w, mut ws_r) = ws_stream.split();

    let store:    Store   = Arc::new(Mutex::new(HashMap::new()));
    let order:    Order   = Arc::new(Mutex::new(Vec::new()));
    let pending:  Pending = Arc::new(Mutex::new(Vec::new()));
    let err_flag: ErrFlag = Arc::new(Mutex::new(None));

    let (s2, o2, p2, n2, f2, ef2) = (
        store.clone(), order.clone(), pending.clone(),
        nick.clone(), friend.clone(), err_flag.clone(),
    );

    // ── Receive task ──────────────────────────────────────────────────────────
    tokio::spawn(async move {
        while let Some(Ok(raw)) = ws_r.next().await {
            if let Ok(text) = raw.into_text() {
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    // Check for fatal error (auth failure etc.)
                    if val["type"].as_str() == Some("error") {
                        let msg = val["text"].as_str().unwrap_or("Unknown error").to_string();
                        *ef2.lock().unwrap() = Some(msg.clone());
                        print!("\r\x1b[K");
                        println!("\n  {RE}✗  {msg}{R}");
                        println!("  {DG}  Make sure both sides use the exact same nick, friend, and room key.{R}\n");
                        io::stdout().flush().unwrap();
                        std::process::exit(1);
                    }

                    print!("\r\x1b[K");
                    io::stdout().flush().unwrap();
                    handle_in(&val, &n2, &f2, &s2, &o2, &p2);
                    print!("{DG}  ›{R} ");
                    io::stdout().flush().unwrap();
                }
            }
        }
        // Connection closed by server
        println!("\n  {RE}✗  Disconnected from server.{R}\n");
        std::process::exit(1);
    });

    // ── Input loop ────────────────────────────────────────────────────────────
    let mut buf = String::new();
    loop {
        print!("{DG}  ›{R} ");
        io::stdout().flush().unwrap();

        buf.clear();
        if io::stdin().read_line(&mut buf).is_err() {
            break;
        }
        let line = buf.trim().to_string();
        if line.is_empty() {
            continue;
        }

        // Erase the echoed input line
        print!("\x1b[1A\r\x1b[K");
        io::stdout().flush().unwrap();

        match line.as_str() {
            "exit" | "/exit" | "/quit" => {
                print!("  Leave? [y/N] ");
                io::stdout().flush().unwrap();
                let mut c = String::new();
                io::stdin().read_line(&mut c).unwrap();
                if c.trim().eq_ignore_ascii_case("y") {
                    println!("\n  {CY}Goodbye! 👋{R}\n");
                    break;
                }
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
                // Optimistic render: show message immediately, ack replaces local id
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
    }
}

// ── Handle incoming WebSocket message ────────────────────────────────────────
fn handle_in(
    val: &Value,
    me: &str,
    _friend: &str,
    store: &Store,
    order: &Order,
    pending: &Pending,
) {
    match sv(val, "type").as_str() {
        // Ack for our sent message — update local id silently
        "ack" => {
            let rid = sv(val, "msg_id");
            let rts = val["timestamp"].as_i64().unwrap_or(0);
            let lid = {
                let mut p = pending.lock().unwrap();
                if p.is_empty() { return; }
                p.remove(0)
            };
            let mut st = store.lock().unwrap();
            let mut o  = order.lock().unwrap();
            if let Some(pos) = o.iter().position(|x| x == &lid) {
                o[pos] = rid.clone();
            }
            if let Some(mut m) = st.remove(&lid) {
                m.msg_id = rid.clone();
                m.ts = rts;
                st.insert(rid, m);
            }
        }

        // History on join
        "history" => {
            let (id, nick, text, ts, edited) = fields(val);
            if !store.lock().unwrap().contains_key(&id) {
                let idx = upsert(store, order, Msg { msg_id: id, nick: nick.clone(), text: text.clone(), ts, edited });
                bubble(idx, &nick, &text, ts, edited, me);
            }
        }

        // New message from friend
        "msg" => {
            let (id, nick, text, ts, edited) = fields(val);
            let idx = upsert(store, order, Msg { msg_id: id, nick: nick.clone(), text: text.clone(), ts, edited });
            bubble(idx, &nick, &text, ts, edited, me);
        }

        "edit" => {
            let (id, txt) = (sv(val, "msg_id"), sv(val, "text"));
            let (nick, ts, idx) = {
                let mut st = store.lock().unwrap();
                let o = order.lock().unwrap();
                match st.get_mut(&id) {
                    Some(m) => {
                        m.text = txt.clone();
                        m.edited = true;
                        let i = pos(&o, &id);
                        (m.nick.clone(), m.ts, i)
                    }
                    None => return,
                }
            };
            bubble(idx, &nick, &txt, ts, true, me);
        }

        "delete" => {
            let id = sv(val, "msg_id");
            let (nick, ts, idx) = {
                let st = store.lock().unwrap();
                let o  = order.lock().unwrap();
                match st.get(&id) {
                    Some(m) => (m.nick.clone(), m.ts, pos(&o, &id)),
                    None => return,
                }
            };
            bubble(idx, &nick, "[ deleted ]", ts, false, me);
        }

        "system" => {
            let text = sv(val, "text");
            let tw   = twidth();
            let pad  = tw.saturating_sub(text.len() + 6) / 2;
            println!("\n{}{CY}── {text} ──{R}\n", " ".repeat(pad));
        }

        "error" => println!("\n  {RE}✗  {}{R}\n", sv(val, "text")),

        _ => {}
    }
}

// ── Render chat bubble ────────────────────────────────────────────────────────
//
//  YOUR message (WHITE) — pinned to RIGHT edge
//  FRIEND message (YELLOW) — pinned to LEFT edge

fn bubble(idx: usize, nick: &str, text: &str, ts: i64, edited: bool, me: &str) {
    let tw      = twidth();
    let is_mine = nick == me;
    let color   = if is_mine { W } else { Y };
    let time    = ftime(ts);

    let lines = wraptext(text, 40);

    let min_inner = nick.len() + time.len() + 4;
    let inner = lines.iter().map(|l| l.len()).max().unwrap_or(0)
        .max(min_inner)
        .min(40);
    let bw = inner + 4;

    // Header: ┌─ nick  HH:MM ──────┐
    let head_text_len = 2 + nick.len() + 2 + time.len() + 1;
    let head_dashes   = bw.saturating_sub(head_text_len + 2);
    let header = format!(
        "{DG}┌─ {color}{nick}{DG}  {time}{}{DG}─┐{R}",
        "─".repeat(head_dashes)
    );

    // Body lines
    let body: Vec<String> = lines.iter().map(|l| {
        let pad = inner.saturating_sub(l.len());
        format!("{DG}│{R} {color}{l}{R}{} {DG}│{R}", " ".repeat(pad))
    }).collect();

    // Edited tag
    let etag: Option<String> = if edited {
        let pad = inner.saturating_sub(8);
        Some(format!("{DG}│{R} {DG}✎ edited{R}{} {DG}│{R}", " ".repeat(pad)))
    } else { None };

    // Footer: └────────[N]──┘
    let itag  = format!("[{idx}]");
    let fdash = bw.saturating_sub(itag.len() + 4);
    let footer = format!("{DG}└{}{}──┘{R}", "─".repeat(fdash), itag);

    let indent = if is_mine { tw.saturating_sub(bw) } else { 2 };
    let pad    = " ".repeat(indent);

    println!("{pad}{header}");
    for l in &body { println!("{pad}{l}"); }
    if let Some(e) = etag { println!("{pad}{e}"); }
    println!("{pad}{footer}");
}

fn history(store: &Store, order: &Order, me: &str) {
    let tw = twidth();
    println!("\n{DG}{}{R}", "─".repeat(tw));
    println!("{CY}  ── History ──{R}");
    println!("{DG}{}{R}\n", "─".repeat(tw));
    let st = store.lock().unwrap();
    let o  = order.lock().unwrap();
    for (i, id) in o.iter().enumerate() {
        if let Some(m) = st.get(id) {
            bubble(i + 1, &m.nick, &m.text, m.ts, m.edited, me);
        }
    }
    println!();
}

// ── UI chrome ─────────────────────────────────────────────────────────────────
fn banner() {
    println!("{CY}");
    println!("  ██╗   ██╗██╗██╗   ██╗  █████╗ ");
    println!("  ██║   ██║██║██║   ██║ ██╔══██╗");
    println!("  ██║   ██║██║██║   ██║ ███████║");
    println!("  ╚██╗ ██╔╝██║╚██╗ ██╔╝ ██╔══██║");
    println!("   ╚████╔╝ ██║ ╚████╔╝  ██║  ██║");
    println!("    ╚═══╝  ╚═╝  ╚═══╝   ╚═╝  ╚═╝");
    println!("{DG}  secret room terminal chat{R}\n");
}

fn help() {
    let tw = twidth();
    println!("{DG}{}{R}", "─".repeat(tw));
    println!("  {DG}Commands:{R}");
    println!("    {DG}/history            {R}— show past messages");
    println!("    {DG}/edit <N> <text>    {R}— edit your message #N");
    println!("    {DG}/delete <N>         {R}— delete your message #N");
    println!("    {DG}exit                {R}— leave chat");
    println!("{DG}{}{R}\n", "─".repeat(tw));
}

fn err(m: &str) { println!("  {RE}⚠  {m}{R}"); }

// ── Utils ─────────────────────────────────────────────────────────────────────
fn upsert(store: &Store, order: &Order, m: Msg) -> usize {
    let mut st = store.lock().unwrap();
    let mut o  = order.lock().unwrap();
    let id = m.msg_id.clone();
    if !st.contains_key(&id) { o.push(id.clone()); }
    let idx = o.iter().position(|x| x == &id).unwrap_or(0) + 1;
    st.insert(id, m);
    idx
}

fn mine(input: &str, order: &Order, store: &Store, nick: &str) -> Option<String> {
    let o  = order.lock().unwrap();
    let st = store.lock().unwrap();
    if let Ok(n) = input.parse::<usize>() {
        if n >= 1 && n <= o.len() {
            let id = &o[n - 1];
            if let Some(m) = st.get(id) {
                if m.nick == nick { return Some(id.clone()); }
            }
        }
        return None;
    }
    let hits: Vec<&String> = o.iter().filter(|id| id.starts_with(input)).collect();
    if hits.len() == 1 {
        if let Some(m) = st.get(hits[0]) {
            if m.nick == nick { return Some(hits[0].clone()); }
        }
    }
    None
}

async fn send<S>(ws: &mut S, v: Value)
where
    S: futures::Sink<tokio_tungstenite::tungstenite::Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    let _ = ws.send(tokio_tungstenite::tungstenite::Message::Text(v.to_string())).await;
}

fn fields(val: &Value) -> (String, String, String, i64, bool) {
    (
        sv(val, "msg_id"),
        sv(val, "nick"),
        sv(val, "text"),
        val["timestamp"].as_i64().unwrap_or(0),
        val["edited"].as_bool().unwrap_or(false),
    )
}

fn pos(order: &[String], id: &str) -> usize {
    order.iter().position(|x| x == id).unwrap_or(0) + 1
}

fn wraptext(text: &str, max: usize) -> Vec<String> {
    if text.len() <= max { return vec![text.to_string()]; }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur = word.to_string();
        } else if cur.len() + 1 + word.len() <= max {
            cur.push(' ');
            cur.push_str(word);
        } else {
            out.push(cur.clone());
            cur = word.to_string();
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

fn sv(val: &Value, key: &str) -> String {
    val[key].as_str().unwrap_or("").to_string()
}

fn ftime(ts: i64) -> String {
    if ts == 0 { return "".to_string(); }
    Local.timestamp_millis_opt(ts)
        .single()
        .unwrap_or_else(Local::now)
        .format("%H:%M")
        .to_string()
}

fn twidth() -> usize {
    terminal_size().map(|(Width(w), _)| w as usize).unwrap_or(100)
}

fn ask(label: &str) -> String {
    print!("{label}");
    io::stdout().flush().unwrap();
    let mut b = String::new();
    io::stdin().read_line(&mut b).unwrap();
    b.trim().to_string()
}

fn clear() {
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();
}

fn enc(s: &str) -> String {
    s.chars().map(|c| match c {
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        _ => format!("%{:02X}", c as u32),
    }).collect()
}

fn ns() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}