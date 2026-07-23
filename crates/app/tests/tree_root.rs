//! `TreeRoot` polymorphic-tree-fn coverage.
//!
//! `launch`'s second argument accepts both `FnOnce(&mut Cx) -> NodeId`
//! and `NodeBuilder` via the sealed `TreeRoot` trait. Both paths must
//! install correctly onto a fresh `Cx` and report the right root.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use accesskit as _;
use crossbeam_channel as _;
use muda as _;
use notify as _;
use raw_window_handle as _;
use rfd as _;
use sigil_a11y as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_layout as _;
use sigil_renderer as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_tasks as _;
use sigil_text as _;
use toml as _;
use tracing as _;
use tray_icon as _;
#[cfg(windows)]
use windows_core as _;

use sigil_kernel::prelude::*;
use sigil_reactive::Runtime;

use sigil_app::TreeRoot;

#[test]
fn fn_root_returns_installed_node_id() {
    Runtime::scope("test", || {
        let mut cx = Cx::new_root();
        let root = (|cx: &mut Cx| cx.add(container())).install(&mut cx);
        assert!(cx.get(root).is_some());
    });
}

#[test]
fn builder_root_installs_and_returns_id() {
    Runtime::scope("test", || {
        let mut cx = Cx::new_root();
        let builder = container();
        let root = builder.install(&mut cx);
        assert!(cx.get(root).is_some());
    });
}

#[test]
fn fn_root_can_capture_state() {
    // Realistic pattern: the tree fn closes over a signal so an
    // event handler installed elsewhere can write into it.
    Runtime::scope("test", || {
        let captured = std::cell::Cell::new(0_u32);
        let mut cx = Cx::new_root();
        let root = (|cx: &mut Cx| {
            captured.set(42);
            cx.add(container())
        })
        .install(&mut cx);
        assert_eq!(captured.get(), 42);
        assert!(cx.get(root).is_some());
    });
}
