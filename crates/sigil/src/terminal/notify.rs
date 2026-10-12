use std::io::{self, Write};

use crate::terminal::caps::{Caps, Notify};
use crate::terminal::graphics::write_mux;
use crate::terminal::term::{current_caps, live};

pub fn notify(title: &str, body: &str) -> io::Result<()> {
    if !live() {
        return Ok(());
    }
    let caps = current_caps();
    notice(caps, title, body).map_or(Ok(()), |seq| send(caps, &seq))
}

pub fn progress(value: Option<u8>) -> io::Result<()> {
    let caps = current_caps();
    if !live() || !caps.notices.progress {
        return Ok(());
    }
    let seq = value.map_or_else(
        || "\x1b]9;4;0\x07".to_owned(),
        |percent| format!("\x1b]9;4;1;{}\x07", percent.min(100)),
    );
    send(caps, &seq)
}

pub fn bell(note: Option<&str>) -> io::Result<()> {
    if !live() {
        return Ok(());
    }
    let caps = current_caps();
    let mut out = io::stdout().lock();
    if let Some(seq) = note.and_then(|note| notice(caps, "", note)) {
        write_mux(&mut out, &seq, caps.mux)?;
    }
    out.write_all(b"\x07")?;
    out.flush()
}

fn notice(caps: Caps, title: &str, body: &str) -> Option<String> {
    let (title, body) = (clean(title), clean(body));
    Some(match caps.notices.notify? {
        Notify::Osc777 => format!(
            "\x1b]777;notify;{};{}\x07",
            title.replace(';', ","),
            body.replace(';', ",")
        ),
        Notify::Osc9 if title.is_empty() => format!("\x1b]9;{body}\x07"),
        Notify::Osc9 => format!("\x1b]9;{title}: {body}\x07"),
    })
}

fn clean(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_control()).collect()
}

fn send(caps: Caps, seq: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    write_mux(&mut out, seq, caps.mux)?;
    out.flush()
}
