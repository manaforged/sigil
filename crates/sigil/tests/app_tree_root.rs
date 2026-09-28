#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::kernel::prelude::*;
use sigil::reactive::{Runtime, derived, signal};

use sigil::app::{TreeRoot, root};

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
    Runtime::scope("test", || {
        let captured = std::rc::Rc::new(std::cell::Cell::new(0_u32));
        let inner = std::rc::Rc::clone(&captured);
        let mut cx = Cx::new_root();
        let root = (move |cx: &mut Cx| {
            inner.set(42);
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
        let root = (move |cx: &mut Cx| cx.add(text(label))).install(&mut cx);
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
