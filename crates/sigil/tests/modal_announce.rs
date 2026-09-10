#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used)]

use sigil::prelude::*;
use sigil_kernel::{AccessibleName, Live};

#[test]
fn modal_announces_title_on_open_rising_edge() {
    sigil::reactive::Runtime::scope("sigil-widget-modal-open-announce", || {
        let mut cx = Cx::new_root();
        let visible = signal(false);
        let _ = cx.add(modal(visible).title("File saved"));

        cx.flush_announcements();
        assert!(
            cx.live_region_id().is_none(),
            "modal that mounted closed must not create a live region before opening"
        );

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
        let _ = cx.add(modal(visible).title("First open"));

        visible.set(true);
        cx.flush_announcements();
        let id = cx
            .live_region_id()
            .expect("rising edge created live region");
        if let Some(node) = cx.get_mut(id) {
            node.a11y.name = AccessibleName::Static(String::from("<sentinel>"));
        }

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
        let _ = cx.add(modal(visible).title("Reopen me"));

        visible.set(true);
        cx.flush_announcements();
        let id = cx.live_region_id().expect("first open created live region");

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
        let _ = cx.add(modal(visible));

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
        let visible = signal(true);
        let _ = cx.add(modal(visible).title("Mounted open"));

        cx.flush_announcements();
        assert!(
            cx.live_region_id().is_none(),
            "modal opened at mount must not announce; live region should not exist yet"
        );
    });
}
