//! Wave 1.3.B — modal rising-edge live-region announcement.
//!
//! Verifies that flipping a modal's `show` signal from `false` to `true`
//! queues a polite live-region announcement carrying the dialog title,
//! and that the announcement materialises on `Cx` once the host drains
//! the pending queue via [`Cx::flush_announcements`]. Steady-state and
//! initial-mount-with-show-true do NOT announce — only the rising edge.

#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used)]

// Workspace-shape integration-test dev-deps: present but unused here.
use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_bidi as _;
use wgpu as _;

use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;

use sigil::prelude::*;
use sigil_kernel::{AccessibleName, Live};

#[test]
fn modal_announces_title_on_open_rising_edge() {
    sigil::reactive::Runtime::scope("sigil-widget-modal-open-announce", || {
        let mut cx = Cx::new_root();
        let visible = signal(false);
        let _ = cx.add(modal().title("File saved").show(visible));

        // Initial mount with `show == false` must NOT announce.
        cx.flush_announcements();
        assert!(
            cx.live_region_id().is_none(),
            "modal that mounted closed must not create a live region before opening"
        );

        // Rising edge: false → true. Effect queues "File saved" polite.
        visible.set(true);
        cx.flush_announcements();
        let id = cx
            .live_region_id()
            .expect("rising edge should have queued a polite announcement");
        let node = cx.get(id).expect("live region node exists after drain");
        assert_eq!(node.a11y.live, Some(Live::Polite));
        match &node.a11y.name {
            AccessibleName::Static(text) => assert_eq!(text, "File saved"),
            other => panic!("expected static label, got {other:?}"),
        }
    });
}

#[test]
fn modal_does_not_announce_on_steady_open_writes() {
    sigil::reactive::Runtime::scope("sigil-widget-modal-steady-open", || {
        let mut cx = Cx::new_root();
        let visible = signal(false);
        let _ = cx.add(modal().title("First open").show(visible));

        // Rising edge → announce.
        visible.set(true);
        cx.flush_announcements();
        let id = cx
            .live_region_id()
            .expect("rising edge created live region");
        // Stomp the label so a no-op write would be detectable.
        if let Some(node) = cx.get_mut(id) {
            node.a11y.name = AccessibleName::Static(String::from("<sentinel>"));
        }

        // Steady-state write of `true` again. No rising edge, no
        // announcement. The sentinel label must survive.
        visible.set(true);
        cx.flush_announcements();
        let node = cx.get(id).expect("node still alive");
        match &node.a11y.name {
            AccessibleName::Static(text) => assert_eq!(text, "<sentinel>"),
            other => panic!("expected sentinel preserved, got {other:?}"),
        }
    });
}

#[test]
fn modal_reopens_re_announce() {
    sigil::reactive::Runtime::scope("sigil-widget-modal-reopen", || {
        let mut cx = Cx::new_root();
        let visible = signal(false);
        let _ = cx.add(modal().title("Reopen me").show(visible));

        visible.set(true);
        cx.flush_announcements();
        let id = cx.live_region_id().expect("first open created live region");

        // Close, then re-open. The second rising edge should re-announce.
        visible.set(false);
        cx.flush_announcements();
        if let Some(node) = cx.get_mut(id) {
            node.a11y.name = AccessibleName::Static(String::from("<sentinel>"));
        }

        visible.set(true);
        cx.flush_announcements();
        let node = cx.get(id).expect("live region persists");
        match &node.a11y.name {
            AccessibleName::Static(text) => assert_eq!(text, "Reopen me"),
            other => panic!("expected reopen label, got {other:?}"),
        }
    });
}

#[test]
fn modal_without_title_announces_default_dialog_label() {
    sigil::reactive::Runtime::scope("sigil-widget-modal-default-label", || {
        let mut cx = Cx::new_root();
        let visible = signal(false);
        let _ = cx.add(modal().show(visible));

        visible.set(true);
        cx.flush_announcements();
        let id = cx
            .live_region_id()
            .expect("rising edge created live region");
        let node = cx.get(id).expect("node exists");
        match &node.a11y.name {
            AccessibleName::Static(text) => assert_eq!(text, "Dialog"),
            other => panic!("expected default Dialog label, got {other:?}"),
        }
    });
}

#[test]
fn modal_open_at_mount_does_not_announce() {
    sigil::reactive::Runtime::scope("sigil-widget-modal-mount-open", || {
        let mut cx = Cx::new_root();
        // Signal starts `true` — modal is already open when the page loads.
        // The screen-reader expectation is "don't announce on mount."
        let visible = signal(true);
        let _ = cx.add(modal().title("Mounted open").show(visible));

        cx.flush_announcements();
        assert!(
            cx.live_region_id().is_none(),
            "modal opened at mount must not announce; live region should not exist yet"
        );
    });
}
