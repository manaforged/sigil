use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use super::{Answers, Backdrop, Known, Question, Questions, Version};
use crate::terminal::graphics::Protocol;

const TIMEOUT: Duration = Duration::from_secs(1);

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

pub(crate) fn exchange(
    query: impl FnOnce(&mut std::io::StdoutLock<'_>) -> std::io::Result<()>,
    done: impl Fn(&str) -> bool,
) -> Option<String> {
    use std::io::{IsTerminal, Write};
    let stdin = std::io::stdin();
    if !stdin.is_terminal() || !std::io::stdout().is_terminal() {
        return None;
    }
    let mut out = std::io::stdout().lock();
    query(&mut out).ok()?;
    out.write_all(b"\x1b[c").ok()?;
    out.flush().ok()?;
    drop(out);
    let mut reply = Vec::new();
    let finished = |text: &str| done(text) || da1(text).is_some();
    collect(&stdin, &mut reply, Instant::now() + TIMEOUT, finished).then_some(())?;
    Some(String::from_utf8_lossy(&reply).into_owned())
}

fn queries(ask: Questions, backdrop: bool) -> String {
    let mut text = String::new();
    if ask.has(Question::Graphics) {
        text.push_str("\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\");
    }
    if ask.has(Question::Sync) {
        text.push_str("\x1b[?2026$p");
    }
    if ask.has(Question::Keys) || ask.has(Question::Color) || ask.has(Question::Version) {
        text.push_str("\x1b[?u");
    }
    if ask.has(Question::Version) {
        text.push_str("\x1b[>0q");
    }
    if ask.has(Question::Color) {
        text.push_str("\x1b[38;2;1;2;3m\x1bP$qm\x1b\\\x1b[0m");
    }
    if backdrop {
        text.push_str("\x1b]11;?\x1b\\");
    }
    text.push_str("\x1b[c");
    text
}

fn read_reply(input: &impl AsFd) -> Option<String> {
    let mut reply = Vec::new();
    if !collect(input, &mut reply, Instant::now() + TIMEOUT, |text| {
        da1(text).is_some()
    }) {
        return None;
    }
    Some(String::from_utf8_lossy(&reply).into_owned())
}

fn collect(
    input: &impl AsFd,
    reply: &mut Vec<u8>,
    deadline: Instant,
    done: impl Fn(&str) -> bool,
) -> bool {
    let mut chunk = [0_u8; 256];
    while !done(&String::from_utf8_lossy(reply)) {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            return false;
        };
        if ready(input, left) != Some(true) {
            return false;
        }
        let Ok(read) = rustix::io::read(input, &mut chunk) else {
            return false;
        };
        let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
            return false;
        };
        reply.extend_from_slice(bytes);
    }
    true
}

fn ready(input: &impl AsFd, wait: Duration) -> Option<bool> {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let wait = Timespec::try_from(wait).ok()?;
    let mut fds = [PollFd::new(input, PollFlags::IN)];
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
        truecolor: dcs(reply, "\x1bP1$r").is_some_and(truecolor),
        version: dcs(reply, "\x1bP>|").map_or(Version::Silent, |name| {
            Known::from_version(name).map_or(Version::Unnamed, Version::Named)
        }),
    }
}

fn dcs<'a>(reply: &'a str, intro: &str) -> Option<&'a str> {
    let (_, rest) = reply.split_once(intro)?;
    rest.split('\x1b').next()
}

fn truecolor(body: &str) -> bool {
    let body = body.replace(':', ";");
    body.contains("38;2;1;2;3") || body.contains("38;2;;1;2;3")
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
