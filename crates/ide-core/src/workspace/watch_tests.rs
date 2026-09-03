use super::*;
use std::fs;
use std::time::Duration;

fn drain(watcher: &Watcher, max_wait: Duration) -> Vec<FsEvent> {
    let mut out = Vec::new();
    let deadline = Instant::now() + max_wait;
    while Instant::now() < deadline {
        if let Some(ev) = watcher.wait_timeout(Duration::from_millis(50)) {
            out.push(ev);
        }
    }
    out
}

#[test]
fn watcher_fires_on_create() {
    let dir = tempfile::tempdir().unwrap();
    let watcher = Watcher::start_with_window(dir.path(), Duration::from_millis(20)).unwrap();
    // Brief settle so the OS watcher is ready before the create.
    thread::sleep(Duration::from_millis(50));
    fs::write(dir.path().join("new.rs"), b"hi").unwrap();
    let events = drain(&watcher, Duration::from_millis(500));
    assert!(
        events.iter().any(|e| matches!(
            e,
            FsEvent::Created(p) | FsEvent::Modified(p) if p.ends_with("new.rs")
        )),
        "expected create/modify for new.rs, got {events:?}"
    );
}

#[test]
fn watcher_coalesces_burst() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("hot.rs");
    fs::write(&target, b"0").unwrap();
    let watcher = Watcher::start_with_window(dir.path(), Duration::from_millis(50)).unwrap();
    thread::sleep(Duration::from_millis(50));
    for i in 0..10_u32 {
        fs::write(&target, format!("{i}")).unwrap();
    }
    let events = drain(&watcher, Duration::from_millis(400));
    let hot_events: Vec<_> = events
        .iter()
        .filter(|e| e.primary_path().ends_with("hot.rs"))
        .collect();
    // 10 bursty writes within a 50ms window collapse to one
    // Modified event for hot.rs. We allow ≤ 2 in case a write
    // straddles a window boundary.
    assert!(
        hot_events.len() <= 2,
        "expected ≤2 coalesced events for hot.rs, got {}: {hot_events:?}",
        hot_events.len()
    );
    assert!(!hot_events.is_empty(), "expected at least one event");
}
