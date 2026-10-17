#![cfg(unix)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use sigil::kernel::text;
use sigil::prelude::AppConfig;
use sigil::reactive::signal;
use sigil::widgets::text_edit;

const CHILD: &str = "SIGIL_TERMINAL_PROBE_CHILD";
const ALTERNATE: &[u8] = b"\x1b[?1049h";
const ERASE_LINE: &[u8] = b"\x1b[2K";

#[test]
fn probe_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let mut out = std::io::stdout();
    out.write_all(b"keep").expect("write");
    out.flush().expect("flush");
    sigil::run(AppConfig::new("probe").size(40, 12).terminal(), || {
        text("inside")
    })
    .expect("run");
}

#[test]
fn editor_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    sigil::run(AppConfig::new("probe").size(40, 12).terminal(), || {
        text_edit(signal(String::new())).autofocus()
    })
    .expect("run");
}

fn position(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn run_child(name: &str, script: impl FnOnce(&Mutex<Box<dyn Write + Send>>)) -> Vec<u8> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 12,
            cols: 40,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("pty");
    let mut command = CommandBuilder::new(std::env::current_exe().expect("test binary"));
    command.args([name, "--exact", "--nocapture", "--test-threads=1"]);
    command.env_clear();
    for (key, value) in [
        ("PATH", "/usr/bin:/bin"),
        ("LANG", "en_US.UTF-8"),
        ("TERM", "xterm-256color"),
        (CHILD, "1"),
    ] {
        command.env(key, value);
    }
    let mut child = pair.slave.spawn_command(command).expect("spawn");
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().expect("reader");
    let writer = Arc::new(Mutex::new(pair.master.take_writer().expect("writer")));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (sink, replies) = (Arc::clone(&seen), Arc::clone(&writer));
    std::thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        while let Ok(read) = reader.read(&mut chunk) {
            let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
                break;
            };
            if position(bytes, b"\x1b[c").is_some() {
                replies.lock().unwrap().write_all(b"\x1b[?62;22c").ok();
            }
            sink.lock().unwrap().extend_from_slice(bytes);
        }
    });
    std::thread::sleep(Duration::from_secs(2));
    script(&writer);
    writer.lock().unwrap().write_all(b"\x03").ok();
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    child.kill().ok();
    std::thread::sleep(Duration::from_millis(200));
    seen.lock().unwrap().clone()
}

#[test]
fn startup_probing_leaves_the_callers_screen_alone() {
    let output = run_child("probe_child", |_| {});
    let entered = position(&output, ALTERNATE).expect("the app entered the alternate screen");
    let erased = position(&output, ERASE_LINE);
    assert!(
        erased.is_none_or(|at| at > entered),
        "a line was erased on the caller's screen before the alternate screen: {:?}",
        String::from_utf8_lossy(&output)
    );
    let mut screen = vt100::Parser::new(12, 40, 0);
    screen.process(&output);
    assert!(
        screen.screen().contents().contains("keep"),
        "the caller's text is gone after exit: {:?}",
        screen.screen().contents()
    );
}

#[test]
fn a_reply_split_across_reads_is_not_typed() {
    let output = run_child("editor_child", |writer| {
        for part in [&b"\x1bP>|xter"[..], b"m(390)\x1b\\", b"ok"] {
            writer.lock().unwrap().write_all(part).ok();
            std::thread::sleep(Duration::from_millis(200));
        }
        std::thread::sleep(Duration::from_millis(300));
    });
    let mut screen = vt100::Parser::new(12, 40, 0);
    let entered = position(&output, ALTERNATE).expect("the app entered the alternate screen");
    screen.process(&output[entered..]);
    let shown = screen.screen().contents();
    assert!(
        shown.contains("ok"),
        "the typed text never arrived: {shown:?}"
    );
    assert!(
        !shown.contains("390"),
        "a split terminal reply was typed: {shown:?}"
    );
}
