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
use sigil::kernel::{ImageSource, image};
use sigil::layout::LayoutStyle;
use sigil::prelude::AppConfig;
use sigil::widgets::layout_ext::WidgetLayoutExt;

const CHILD: &str = "SIGIL_TERMINAL_IMAGES_CHILD";

#[test]
fn image_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let pixels: Vec<u8> = (0..=255_u8)
        .flat_map(|index| [index.wrapping_mul(7), 40, 200, 255])
        .collect();
    let source = ImageSource::from_rgba8(16, 16, pixels).expect("image");
    sigil::run(
        AppConfig::new("images").size(40, 12).terminal(),
        move || {
            image(source.clone()).layout(LayoutStyle::block().fixed_size(16.0, 8.0).shrink(0.0))
        },
    )
    .expect("run");
}

fn find(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn sixel(output: &[u8]) -> bool {
    output.windows(2).enumerate().any(|(start, pair)| {
        pair == b"\x1bP"
            && output.get(start + 2..).is_some_and(|rest| {
                rest.iter()
                    .find(|byte| !(byte.is_ascii_digit() || **byte == b';'))
                    == Some(&b'q')
            })
    })
}

fn image_escapes(output: &[u8]) -> bool {
    find(output, b"\x1b_G") || find(output, b"\x1b]1337;File") || sixel(output)
}

fn block_glyphs(output: &[u8]) -> bool {
    String::from_utf8_lossy(output)
        .chars()
        .any(|glyph| matches!(glyph, '\u{2580}'..='\u{259f}' | '\u{1fb00}'..='\u{1fb3b}'))
}

type Writer = Arc<Mutex<Box<dyn Write + Send>>>;

fn answer(writer: &Writer, bytes: &[u8]) {
    let mut out = writer.lock().unwrap();
    if find(bytes, b"a=q") {
        out.write_all(b"\x1b_Gi=31;OK\x1b\\").ok();
    }
    if find(bytes, b"\x1b[c") {
        out.write_all(b"\x1b[?62;4;22c").ok();
    }
}

fn spawn_child(
    env: &[(&str, &str)],
) -> (
    Box<dyn portable_pty::Child + Send + Sync>,
    Box<dyn Read + Send>,
    Writer,
) {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 12,
            cols: 40,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("pty");
    let mut command = CommandBuilder::new(std::env::current_exe().expect("test binary"));
    command.args(["image_child", "--exact", "--nocapture", "--test-threads=1"]);
    command.env_clear();
    for (key, value) in [
        ("PATH", "/usr/bin:/bin"),
        ("LANG", "en_US.UTF-8"),
        (CHILD, "1"),
    ]
    .iter()
    .chain(env)
    {
        command.env(key, value);
    }
    let child = pair.slave.spawn_command(command).expect("spawn");
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().expect("reader");
    let writer = Arc::new(Mutex::new(pair.master.take_writer().expect("writer")));
    (child, reader, writer)
}

fn run_child(env: &[(&str, &str)]) -> Vec<u8> {
    let (mut child, mut reader, writer) = spawn_child(env);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (sink, replies) = (Arc::clone(&seen), Arc::clone(&writer));
    std::thread::spawn(move || {
        let mut chunk = [0_u8; 4096];
        while let Ok(read) = reader.read(&mut chunk) {
            let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
                break;
            };
            answer(&replies, bytes);
            sink.lock().unwrap().extend_from_slice(bytes);
        }
    });
    std::thread::sleep(Duration::from_secs(3));
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
fn a_terminal_the_environment_does_not_name_gets_glyphs_not_image_escapes() {
    let output = run_child(&[("TERM", "xterm-256color"), ("TERM_PROGRAM", "mux")]);
    assert!(
        !image_escapes(&output),
        "image escape written to an unnamed terminal: {:?}",
        String::from_utf8_lossy(&output)
    );
    assert!(
        block_glyphs(&output),
        "image not drawn as glyphs: {:?}",
        String::from_utf8_lossy(&output)
    );
}

#[test]
fn a_kitty_term_gets_kitty_images() {
    let output = run_child(&[("TERM", "xterm-kitty")]);
    assert!(
        find(&output, b"\x1b_Ga=t"),
        "no Kitty image for TERM=xterm-kitty: {:?}",
        String::from_utf8_lossy(&output)
    );
}
