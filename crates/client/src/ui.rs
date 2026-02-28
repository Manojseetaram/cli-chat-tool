use crate::{
    colors::*,
    types::{Order, Store},
    utils::{ftime, twidth, wraptext},
};

pub fn banner() {
    println!("{CY}");
    println!("  ██╗   ██╗██╗██╗   ██╗  █████╗ ");
    println!("  ██║   ██║██║██║   ██║ ██╔══██╗");
    println!("  ██║   ██║██║██║   ██║ ███████║");
    println!("  ╚██╗ ██╔╝██║╚██╗ ██╔╝ ██╔══██║");
    println!("   ╚████╔╝ ██║ ╚████╔╝  ██║  ██║");
    println!("    ╚═══╝  ╚═╝  ╚═══╝   ╚═╝  ╚═╝");
    println!("{DG}  secret room terminal chat{R}\n");
}

pub fn help() {
    let tw = twidth();
    println!("{DG}{}{R}", "─".repeat(tw));
    println!("  {DG}Commands:{R}");
    println!("    {DG}/history            {R}— show past messages");
    println!("    {DG}/edit <N> <text>    {R}— edit your message #N");
    println!("    {DG}/delete <N>         {R}— delete your message #N");
    println!("    {DG}exit  /  bye        {R}— leave chat");
    println!("{DG}{}{R}\n", "─".repeat(tw));
}

pub fn err(m: &str) {
    println!("  {RE}⚠  {m}{R}");
}

pub fn bubble(idx: usize, nick: &str, text: &str, ts: i64, edited: bool, me: &str) {
    let tw = twidth();
    let is_mine = nick == me;
    let color = if is_mine { W } else { Y };
    let time = ftime(ts);
    let lines = wraptext(text, 40);
    let min_inner = nick.len() + time.len() + 4;
    let inner = lines
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(0)
        .max(min_inner)
        .min(40);
    let bw = inner + 4;
    let head_text_len = 2 + nick.len() + 2 + time.len() + 1;
    let head_dashes = bw.saturating_sub(head_text_len + 2);
    let header = format!(
        "{DG}┌─ {color}{nick}{DG}  {time}{}{DG}─┐{R}",
        "─".repeat(head_dashes)
    );
    let body: Vec<String> = lines
        .iter()
        .map(|l| {
            let pad = inner.saturating_sub(l.len());
            format!("{DG}│{R} {color}{l}{R}{} {DG}│{R}", " ".repeat(pad))
        })
        .collect();
    let etag: Option<String> = if edited {
        let pad = inner.saturating_sub(8);
        Some(format!(
            "{DG}│{R} {DG}✎ edited{R}{} {DG}│{R}",
            " ".repeat(pad)
        ))
    } else {
        None
    };
    let itag = format!("[{idx}]");
    let fdash = bw.saturating_sub(itag.len() + 4);
    let footer = format!("{DG}└{}{}──┘{R}", "─".repeat(fdash), itag);
    let indent = if is_mine { tw.saturating_sub(bw) } else { 2 };
    let pad = " ".repeat(indent);
    println!("{pad}{header}");
    for l in &body {
        println!("{pad}{l}");
    }
    if let Some(e) = etag {
        println!("{pad}{e}");
    }
    println!("{pad}{footer}");
}

pub fn history(store: &Store, order: &Order, me: &str) {
    let tw = twidth();
    println!("\n{DG}{}{R}", "─".repeat(tw));
    println!("{CY}  ── History ──{R}");
    println!("{DG}{}{R}\n", "─".repeat(tw));
    let st = store.lock().unwrap();
    let o = order.lock().unwrap();
    for (i, id) in o.iter().enumerate() {
        if let Some(m) = st.get(id) {
            bubble(i + 1, &m.nick, &m.text, m.ts, m.edited, me);
        }
    }
    println!();
}