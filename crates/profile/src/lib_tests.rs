use std::sync::{Mutex, MutexGuard, OnceLock};

use super::*;

fn lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn disabled_span_is_silent() {
    let _gate = lock();
    disable();
    assert!(!enabled());
    {
        let _s = span("layout");
    }
    assert!(frames().is_empty());
}

#[test]
fn span_records_when_enabled() {
    let _gate = lock();
    enable();
    reset();
    begin_frame();
    {
        let _s = span("layout");
        count("text.measure");
    }
    tick_frame();
    let got = frames();
    let last = got.last().expect("frame");
    assert!(
        last.phases.iter().any(|(n, _)| *n == "layout"),
        "layout phase present"
    );
    assert_eq!(last.counter("text.measure"), 1);
    disable();
}

#[test]
fn pointer_beats_hover_on_the_same_frame() {
    let _gate = lock();
    enable();
    reset();
    begin_frame();
    interact("hover");
    interact("pointer");
    tick_frame();
    let got = frames();
    let last = got.last().expect("frame");
    assert_eq!(last.cause, "pointer");
    disable();
}

#[test]
fn count_add_sums() {
    let _gate = lock();
    enable();
    reset();
    begin_frame();
    count_add("scene.cmds", 12);
    count_add("scene.cmds", 3);
    tick_frame();
    assert_eq!(frames().last().expect("frame").counter("scene.cmds"), 15);
    disable();
}

#[test]
fn dump_writes_chrome_json() {
    let _gate = lock();
    enable();
    reset();
    begin_frame();
    interact("pointer");
    {
        let _s = span("paint");
        count("scene.emit");
    }
    let path = std::env::temp_dir().join("sigil-profile-dump-test.json");
    dump(&path).expect("dump");
    let json = std::fs::read_to_string(&path).expect("read dump");
    assert!(json.contains("\"traceEvents\""), "chrome envelope");
    assert!(json.contains("process_name"), "metadata");
    assert!(json.contains("\"name\":\"paint\""), "span name");
    assert!(json.contains("\"ph\":\"i\""), "input instant");
    assert!(json.contains("\"ph\":\"s\""), "flow start");
    assert!(json.contains("\"cat\":\"input\""), "input category");
    let text = summary();
    assert!(text.contains("paint"), "summary names paint");
    assert!(text.contains("pointer"), "summary names cause");
    assert!(
        text.lines()
            .any(|line| line.contains("scene.emit") && line.contains('1')),
        "counts print as integers: {text}"
    );
    disable();
    drop(std::fs::remove_file(path));
}

#[test]
fn dump_closes_open_frame() {
    let _gate = lock();
    enable();
    reset();
    {
        let _s = span("effect");
    }
    assert!(frames().is_empty(), "not ticked yet");
    let path = std::env::temp_dir().join("sigil-profile-close-test.json");
    dump(&path).expect("dump");
    assert_eq!(frames().len(), 1, "open frame flushed");
    disable();
    drop(std::fs::remove_file(path));
}
