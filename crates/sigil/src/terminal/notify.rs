use std::io::{self, Write};
use std::sync::{Mutex, PoisonError};

use crate::terminal::caps::{Caps, Notify};
use crate::terminal::graphics::write_mux;
use crate::terminal::term::{applied, current_caps, live};

/// Sends a desktop notification through the terminal.
pub fn notify(title: &str, body: &str) -> Result<(), crate::Error> {
    if !applied() {
        return Ok(());
    }
    let caps = current_caps();
    notice(caps, title, body).map_or(Ok(()), |seq| send(caps, &seq))
}

/// Reports task progress to the terminal.
pub fn progress(value: Option<u8>) -> Result<(), crate::Error> {
    let caps = current_caps();
    if !applied() || !caps.notices.progress {
        return Ok(());
    }
    let seq = value.map_or_else(
        || "\x1b]9;4;0\x07".to_owned(),
        |percent| format!("\x1b]9;4;1;{}\x07", percent.min(100)),
    );
    send(caps, &seq)
}

/// What a program reports to a host that lists agents: waiting, running, or waiting on the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// Waiting for input.
    Idle,
    /// Running a turn.
    Working,
    /// Waiting on the user, such as an approval.
    Blocked,
}

impl Activity {
    const fn wire(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked => "blocked",
        }
    }
}

const CLEAR_ACTIVITY: &str = "\x1b]7501;state=clear\x1b\\";

static ACTIVITY: Mutex<String> = Mutex::new(String::new());

/// Reports `app`'s activity to the host over OSC 7501; `None` clears it.
pub fn activity(app: &str, value: Option<Activity>) -> Result<(), crate::Error> {
    let caps = current_caps();
    if !live() || !caps.notices.activity {
        return Ok(());
    }
    let seq = value.map_or_else(String::new, |value| activity_sequence(app, value));
    ACTIVITY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone_from(&seq);
    if !applied() {
        return Ok(());
    }
    send(caps, if seq.is_empty() { CLEAR_ACTIVITY } else { &seq })
}

fn activity_sequence(app: &str, value: Activity) -> String {
    let app: String = app
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '+' | '-') {
                ch
            } else {
                '-'
            }
        })
        .take(32)
        .collect();
    format!("\x1b]7501;state={}:app={app}\x1b\\", value.wire())
}

pub(crate) fn held_activity(shown: bool) -> Option<String> {
    let held = ACTIVITY.lock().unwrap_or_else(PoisonError::into_inner);
    if held.is_empty() {
        None
    } else if shown {
        Some(held.clone())
    } else {
        Some(CLEAR_ACTIVITY.to_owned())
    }
}

/// Rings the terminal bell.
pub fn bell(note: Option<&str>) -> Result<(), crate::Error> {
    if !applied() {
        return Ok(());
    }
    let caps = current_caps();
    let mut out = io::stdout().lock();
    if let Some(seq) = note.and_then(|note| notice(caps, "", note)) {
        write_mux(&mut out, &seq, caps.mux)?;
    }
    out.write_all(b"\x07")?;
    out.flush().map_err(Into::into)
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

fn send(caps: Caps, seq: &str) -> Result<(), crate::Error> {
    let mut out = io::stdout().lock();
    write_mux(&mut out, seq, caps.mux)?;
    out.flush().map_err(Into::into)
}

#[cfg(test)]
#[path = "notify_tests.rs"]
mod tests;
