use chrono::{Local, TimeZone};
use std::io::{self, Write};
use terminal_size::{terminal_size, Width};

pub fn ask(label: &str) -> String {
    print!("{label}");
    io::stdout().flush().unwrap();
    let mut b = String::new();
    io::stdin().read_line(&mut b).unwrap();
    b.trim().to_string()
}

pub fn clear() {
    print!("\x1b[2J\x1b[H");
    io::stdout().flush().unwrap();
}

pub fn enc(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

pub fn ftime(ts: i64) -> String {
    if ts == 0 {
        return "".to_string();
    }
    Local
        .timestamp_millis_opt(ts)
        .single()
        .unwrap_or_else(Local::now)
        .format("%H:%M")
        .to_string()
}

pub fn twidth() -> usize {
    terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(100)
}

pub fn wraptext(text: &str, max: usize) -> Vec<String> {
    if text.len() <= max {
        return vec![text.to_string()];
    }
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
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn ns() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}