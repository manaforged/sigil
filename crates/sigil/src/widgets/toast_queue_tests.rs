use std::time::{Duration, Instant};

use super::*;
use crate::kernel::Cx;
use crate::reactive::Runtime;

#[test]
fn push_appends_entries_oldest_first_newest_last() {
    Runtime::scope("toast-queue-push-order", || {
        let queue = ToastQueue::new();
        queue.info("first");
        queue.error("second");
        let visible = queue.visible();
        let summary: Vec<(ToastLevel, &str)> = visible
            .iter()
            .map(|e| (e.level, e.message.as_str()))
            .collect();
        assert_eq!(
            summary,
            [(ToastLevel::Info, "first"), (ToastLevel::Error, "second")],
            "oldest first, newest last"
        );
    });
}

#[test]
fn entries_expire_after_ttl() {
    Runtime::scope("toast-queue-expiry", || {
        let queue = ToastQueue::new();
        queue.success("saved");
        let now = Instant::now();
        queue.dismiss_expired(now);
        assert_eq!(queue.visible().len(), 1, "fresh entry survives a sweep");
        queue.dismiss_expired(now + TOAST_TTL + Duration::from_secs(1));
        assert!(queue.visible().is_empty(), "entry past TTL is dropped");
    });
}

#[test]
fn visible_caps_at_max_visible_dropping_oldest() {
    Runtime::scope("toast-queue-cap", || {
        let queue = ToastQueue::new();
        for n in 0..6 {
            queue.push(ToastLevel::Info, format!("toast {n}"));
        }
        let visible = queue.visible();
        let messages: Vec<&str> = visible.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(messages.len(), MAX_VISIBLE);
        assert_eq!(
            messages,
            ["toast 2", "toast 3", "toast 4", "toast 5"],
            "oldest two dropped, newest kept last"
        );
    });
}

#[test]
fn levels_map_to_indicator_tones() {
    assert_eq!(ToastLevel::Info.tone(), Tone::Info);
    assert_eq!(ToastLevel::Success.tone(), Tone::Success);
    assert_eq!(ToastLevel::Warning.tone(), Tone::Warn);
    assert_eq!(ToastLevel::Error.tone(), Tone::Error);
}

#[test]
fn widget_renders_visible_stack_and_hides_overflow() {
    Runtime::scope("toast-queue-build", || {
        let queue = ToastQueue::new();
        queue.warning("one");
        queue.warning("two");
        let mut cx = Cx::new_root();
        let root = cx.add(toast_queue(queue));
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(root), 2, "one node per visible toast");
        for n in 3..=6 {
            queue.info(format!("toast {n}"));
        }
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(root), MAX_VISIBLE, "overflow not drawn");
    });
}
