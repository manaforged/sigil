use std::time::{Duration, Instant};

use super::*;
use crate::kernel::Cx;
use crate::reactive::Runtime;

#[test]
fn visible_keeps_push_order_and_caps_at_max_visible() {
    Runtime::scope("toast-queue-push-order", || {
        let queue = ToastQueue::new();
        queue.info("toast 0");
        queue.error("toast 1");
        for n in 2..6 {
            queue.push(ToastLevel::Info, format!("toast {n}"));
        }
        queue.error("toast 6");
        let visible = queue.visible();
        let summary: Vec<(ToastLevel, &str)> = visible
            .iter()
            .map(|e| (e.level, e.message.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                (ToastLevel::Info, "toast 3"),
                (ToastLevel::Info, "toast 4"),
                (ToastLevel::Info, "toast 5"),
                (ToastLevel::Error, "toast 6"),
            ],
            "oldest dropped, newest kept last"
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
