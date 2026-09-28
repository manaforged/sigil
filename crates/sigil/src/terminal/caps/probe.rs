use std::time::Duration;

use super::{Answers, Backdrop, Questions};
use crate::terminal::graphics::Protocol;

const TIMEOUT: Duration = Duration::from_millis(150);

pub(super) fn ask(ask: Questions, backdrop: bool) -> Option<Answers> {
    use std::io::{IsTerminal, Write};
    let stdin = std::io::stdin();
    if !stdin.is_terminal() || !std::io::stdout().is_terminal() || ready(&stdin, Duration::ZERO)? {
        return None;
    }
    let mut out = std::io::stdout().lock();
    out.write_all(queries(ask, backdrop).as_bytes()).ok()?;
    out.flush().ok()?;
    drop(out);
    let reply = read_reply(&stdin)?;
    Some(parse(&reply))
}

fn queries(ask: Questions, backdrop: bool) -> String {
    let mut text = String::new();
    if ask.graphics {
        text.push_str("\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\");
    }
    if ask.sync {
        text.push_str("\x1b[?2026$p");
    }
    if ask.keys {
        text.push_str("\x1b[?u");
    }
    if backdrop {
        text.push_str("\x1b]11;?\x1b\\");
    }
    text.push_str("\x1b[c");
    text
}

fn read_reply(stdin: &std::io::Stdin) -> Option<String> {
    let deadline = std::time::Instant::now() + TIMEOUT;
    let mut reply = Vec::new();
    let mut chunk = [0_u8; 256];
    while da1(&String::from_utf8_lossy(&reply)).is_none() {
        let left = deadline.checked_duration_since(std::time::Instant::now())?;
        if !ready(stdin, left)? {
            return None;
        }
        let read = rustix::io::read(stdin, &mut chunk).ok()?;
        if read == 0 {
            return None;
        }
        reply.extend_from_slice(chunk.get(..read)?);
    }
    Some(String::from_utf8_lossy(&reply).into_owned())
}

fn ready(stdin: &std::io::Stdin, wait: Duration) -> Option<bool> {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let wait = Timespec::try_from(wait).ok()?;
    let mut fds = [PollFd::new(stdin, PollFlags::IN)];
    Some(poll(&mut fds, Some(&wait)).ok()? > 0)
}

fn parse(reply: &str) -> Answers {
    Answers {
        graphics: if reply.contains("\x1b_Gi=31;OK") {
            Some(Protocol::Kitty)
        } else if da1(reply).is_some_and(|params| params.split(';').any(|param| param == "4")) {
            Some(Protocol::Sixel)
        } else {
            None
        },
        sync: reply
            .split("\x1b[?2026;")
            .skip(1)
            .any(|rest| rest.starts_with(['1', '2', '3', '4'])),
        kitty_keys: csi_params(reply, 'u').is_some(),
        backdrop: backdrop(reply),
    }
}

fn da1(reply: &str) -> Option<&str> {
    csi_params(reply, 'c')
}

fn csi_params(reply: &str, last: char) -> Option<&str> {
    reply.split("\x1b[?").skip(1).find_map(|rest| {
        let end = rest.find(|ch: char| !(ch.is_ascii_digit() || ch == ';'))?;
        let head = rest.get(..end)?;
        rest.get(end..)?.starts_with(last).then_some(head)
    })
}

fn backdrop(reply: &str) -> Option<Backdrop> {
    let (_, rest) = reply.split_once("\x1b]11;rgb:")?;
    let body = rest.split(['\x1b', '\x07']).next()?;
    let mut channels = body.split('/').map(|hex| {
        let digits = u32::try_from(hex.len()).ok()?;
        let value = u32::from_str_radix(hex, 16).ok()?;
        let top = 16_u32.checked_pow(digits)?.checked_sub(1)?;
        Some(f64::from(value) / f64::from(top.max(1)))
    });
    let (r, g, b) = (channels.next()??, channels.next()??, channels.next()??);
    let luma = 0.0722f64.mul_add(b, 0.2126f64.mul_add(r, 0.7152 * g));
    Some(if luma < 0.5 {
        Backdrop::Dark
    } else {
        Backdrop::Light
    })
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod tests;
