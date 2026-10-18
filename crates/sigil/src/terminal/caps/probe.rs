use std::io::Write;
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use super::{Answers, Backdrop, Known, Question, Questions, Version};
use crate::terminal::term::{ProbeRow, calibration, measure};

const TIMEOUT: Duration = Duration::from_millis(500);

pub(super) fn ask_on(
    input: &impl AsFd,
    out: impl Write,
    ask: Questions,
    (backdrop, widths): (bool, Option<ProbeRow>),
) -> Option<Answers> {
    let mut query = queries(ask, backdrop);
    if let Some(row) = widths {
        query.push_str(&calibration(row));
    }
    let reply = converse(
        input,
        out,
        |batch| batch.write_all(query.as_bytes()),
        |_| false,
    )?;
    Some(parse(&reply))
}

pub(crate) fn exchange(
    query: impl FnOnce(&mut Vec<u8>) -> std::io::Result<()>,
    done: impl Fn(&str) -> bool,
) -> Option<String> {
    converse(&quiet_terminal()?, std::io::stdout().lock(), query, done)
}

pub(super) fn converse(
    input: &impl AsFd,
    mut out: impl Write,
    query: impl FnOnce(&mut Vec<u8>) -> std::io::Result<()>,
    done: impl Fn(&str) -> bool,
) -> Option<String> {
    let mut batch = Vec::new();
    query(&mut batch).ok()?;
    batch.extend_from_slice(b"\x1b[c");
    out.write_all(&batch).ok()?;
    out.flush().ok()?;
    drop(out);
    read_reply(input, done)
}

pub(super) fn quiet_terminal() -> Option<std::io::Stdin> {
    let stdin = std::io::stdin();
    if !terminal() || typing(&stdin)? {
        return None;
    }
    Some(stdin)
}

fn terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn typing(stdin: &impl AsFd) -> Option<bool> {
    Some(
        crossterm::event::poll(Duration::ZERO).unwrap_or(true)
            || ready(stdin, Duration::ZERO).ok()?,
    )
}

fn queries(ask: Questions, backdrop: bool) -> String {
    let mut text = String::new();
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
    text
}

fn read_reply(input: &impl AsFd, done: impl Fn(&str) -> bool) -> Option<String> {
    let deadline = Instant::now() + TIMEOUT;
    let mut reply = Vec::new();
    let mut chunk = [0_u8; 256];
    loop {
        {
            let text = String::from_utf8_lossy(&reply);
            if done(&text) || da1(&text).is_some() {
                return Some(text.into_owned());
            }
        }
        let left = deadline.checked_duration_since(Instant::now())?;
        match ready(input, left) {
            Ok(true) => {}
            Err(rustix::io::Errno::INTR) => continue,
            Ok(false) | Err(_) => return None,
        }
        let read = match rustix::io::read(input, &mut chunk) {
            Ok(read) => read,
            Err(rustix::io::Errno::INTR) => continue,
            Err(_) => return None,
        };
        reply.extend_from_slice(chunk.get(..read).filter(|bytes| !bytes.is_empty())?);
    }
}

fn ready(input: &impl AsFd, wait: Duration) -> rustix::io::Result<bool> {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};
    let wait = Timespec::try_from(wait).map_err(|_| rustix::io::Errno::INVAL)?;
    let mut fds = [PollFd::new(input, PollFlags::IN)];
    Ok(poll(&mut fds, Some(&wait))? > 0)
}

pub(super) fn parse(reply: &str) -> Answers {
    Answers {
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
        measured: measure(&cursor_reports(reply)),
    }
}

fn cursor_reports(reply: &str) -> Vec<(u16, u16)> {
    reply
        .split("\x1b[")
        .skip(1)
        .filter_map(|rest| {
            let (row, rest) = rest.split_once(';')?;
            let (col, _) = rest.split_once('R')?;
            Some((row.parse().ok()?, col.parse::<u16>().ok()?.checked_sub(1)?))
        })
        .collect()
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
