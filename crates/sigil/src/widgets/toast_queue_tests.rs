use std::time::{Duration, Instant};

use super::*;
use crate::kernel::Cx;
use crate::reactive::Owner;

#[test]
fn visible_keeps_push_order_and_caps_at_max_visible() {
    Owner::scope("toast-queue-push-order", || {
        let queue = ToastQueue::new();
        queue.info("toast 0");
        queue.error("toast 1");
        for n in 2..6 {
            queue.push(Tone::Info, format!("toast {n}"));
        }
        queue.error("toast 6");
        let visible = queue.visible();
        let summary: Vec<(Tone, &str)> = visible
            .iter()
            .map(|e| (e.tone, e.message.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                (Tone::Info, "toast 3"),
                (Tone::Info, "toast 4"),
                (Tone::Info, "toast 5"),
                (Tone::Error, "toast 6"),
            ],
            "oldest dropped, newest kept last"
        );
    });
}

#[test]
fn entries_expire_after_ttl() {
    Owner::scope("toast-queue-expiry", || {
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
fn widget_renders_visible_stack_and_hides_overflow() {
    Owner::scope("toast-queue-build", || {
        let queue = ToastQueue::new();
        queue.warning("one");
        queue.warning("two");
        let mut cx = Cx::new_root();
        let root = cx.add(toast_queue(queue));
        let list = cx.first_child(root).expect("list");
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(list), 2, "one node per visible toast");
        crate::animation::runtime::MotionPrefs::set(true);
        for n in 3..=6 {
            queue.info(format!("toast {n}"));
        }
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(list), MAX_VISIBLE, "overflow not drawn");
        crate::animation::runtime::MotionPrefs::set(false);
    });
}
