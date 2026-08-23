#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! `TreeRoot` polymorphic-tree-fn coverage.
//!
//! `launch`'s second argument accepts a sealed `TreeRoot`: a closure,
//! `text`, `container()`, or [`sigil_app::root`] around any `IntoNode`.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil_kernel::prelude::*;
use sigil_reactive::{Runtime, derived, signal};

use sigil_app::{TreeRoot, root};

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

#[test]
fn text_node_is_a_tree_root() {
    Runtime::scope("test", || {
        let mut cx = Cx::new_root();
        let root = text("hello").install(&mut cx);
        assert!(cx.get(root).is_some());
    });
}

#[test]
fn derived_survives_closure_install() {
    Runtime::scope("test", || {
        let n = signal(0_i32);
        let label = derived(move || n.get().to_string());
        let mut cx = Cx::new_root();
        let root = (|cx: &mut Cx| cx.add(text(label))).install(&mut cx);
        n.set(7);
        assert_eq!(label.peek(), "7");
        assert!(cx.get(root).is_some());
    });
}

#[test]
fn boxed_into_node_via_root() {
    Runtime::scope("test", || {
        let mut cx = Cx::new_root();
        let id = root(node(text("hi"))).install(&mut cx);
        assert!(cx.get(id).is_some());
    });
}
