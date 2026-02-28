use serde_json::Value;
use std::io::{self, Write};
use futures::SinkExt;
use crate::{
    colors::*,
    types::{Msg, Order, Pending, Store},
    ui::bubble,
    utils::twidth,
};

pub fn handle_in(
    val: &Value,
    me: &str,
    _friend: &str,
    store: &Store,
    order: &Order,
    pending: &Pending,
) {
    match sv(val, "type").as_str() {
        "ack" => {
            let rid = sv(val, "msg_id");
            let rts = val["timestamp"].as_i64().unwrap_or(0);
            let lid = {
                let mut p = pending.lock().unwrap();
                if p.is_empty() { return; }
                p.remove(0)
            };
            let mut st = store.lock().unwrap();
            let mut o = order.lock().unwrap();
            if let Some(pos) = o.iter().position(|x| x == &lid) {
                o[pos] = rid.clone();
            }
            if let Some(mut m) = st.remove(&lid) {
                m.msg_id = rid.clone();
                m.ts = rts;
                st.insert(rid, m);
            }
        }
        "history" => {
            let (id, nick, text, ts, edited) = fields(val);
            if !store.lock().unwrap().contains_key(&id) {
                let idx = upsert(store, order, Msg { msg_id: id, nick: nick.clone(), text: text.clone(), ts, edited });
                bubble(idx, &nick, &text, ts, edited, me);
            }
        }
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
                let o = order.lock().unwrap();
                match st.get(&id) {
                    Some(m) => (m.nick.clone(), m.ts, pos(&o, &id)),
                    None => return,
                }
            };
            bubble(idx, &nick, "[ deleted ]", ts, false, me);
        }
        "system" => {
            let text = sv(val, "text");
            let tw = twidth();
            let pad = tw.saturating_sub(text.len() + 6) / 2;
            println!("\n{}{CY}── {text} ──{R}\n", " ".repeat(pad));
        }
        "error" => println!("\n  {RE}✗  {}{R}\n", sv(val, "text")),
        _ => {}
    }
}

pub async fn send<S>(ws: &mut S, v: serde_json::Value)
where
    S: futures::Sink<tokio_tungstenite::tungstenite::Message> + Unpin,
    S::Error: std::fmt::Debug,
{
    let _ = ws
        .send(tokio_tungstenite::tungstenite::Message::Text(v.to_string()))
        .await;
}

pub fn upsert(store: &Store, order: &Order, m: Msg) -> usize {
    let mut st = store.lock().unwrap();
    let mut o = order.lock().unwrap();
    let id = m.msg_id.clone();
    if !st.contains_key(&id) {
        o.push(id.clone());
    }
    let idx = o.iter().position(|x| x == &id).unwrap_or(0) + 1;
    st.insert(id, m);
    idx
}

pub fn mine(input: &str, order: &Order, store: &Store, nick: &str) -> Option<String> {
    let o = order.lock().unwrap();
    let st = store.lock().unwrap();
    if let Ok(n) = input.parse::<usize>() {
        if n >= 1 && n <= o.len() {
            let id = &o[n - 1];
            if let Some(m) = st.get(id) {
                if m.nick == nick {
                    return Some(id.clone());
                }
            }
        }
        return None;
    }
    let hits: Vec<&String> = o.iter().filter(|id| id.starts_with(input)).collect();
    if hits.len() == 1 {
        if let Some(m) = st.get(hits[0]) {
            if m.nick == nick {
                return Some(hits[0].clone());
            }
        }
    }
    None
}

// ── Private helpers ───────────────────────────────────────────────────────────

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

pub fn sv(val: &Value, key: &str) -> String {
    val[key].as_str().unwrap_or("").to_string()
}