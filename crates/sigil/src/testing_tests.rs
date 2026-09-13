use super::*;
use crate::reactive::{Signal, signal};

#[test]
fn closure_runs_with_fresh_cx() {
    let mut ran = false;
    with_cx("with-cx-runs", |_cx| {
        ran = true;
    });
    assert!(ran, "closure passed to with_cx must execute");
}

#[test]
fn signal_mutation_inside_scope() {
    with_cx("with-cx-signal", |_cx| {
        let s: Signal<i32> = signal(1);
        s.set(42);
        assert_eq!(s.get(), 42);
    });
}

#[test]
fn with_runtime_runs_closure() {
    let mut ran = false;
    with_runtime("with-runtime-runs", |_| {
        ran = true;
    });
    assert!(ran, "closure passed to with_runtime must execute");
}

#[test]
fn with_runtime_allows_signal_mutation() {
    with_runtime("with-runtime-signal", |_| {
        let s: Signal<i32> = signal(7);
        s.set(99);
        assert_eq!(s.get(), 99);
    });
}

#[test]
fn tree_size_counts_a_single_root() {
    use crate::prelude::container;
    let count = tree_size("tree-size-single", |cx| cx.add(container()));
    assert_eq!(count, 1);
}

#[test]
fn tree_size_counts_children() {
    use crate::prelude::{container, text};
    let count = tree_size("tree-size-children", |cx| {
        cx.add(container().child(text("a")).child(text("b")))
    });
    assert_eq!(count, 3);
}
