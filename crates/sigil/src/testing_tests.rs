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
    let count = tree_size("tree-size-single", container());
    assert_eq!(count, 1);
}

#[test]
fn tree_size_counts_children() {
    use crate::prelude::{container, text};
    let count = tree_size("tree-size-children", || {
        container().child(text("a")).child(text("b"))
    });
    assert_eq!(count, 3);
}

#[test]
fn tick_intervals_fires_a_due_interval_in_a_hand_driven_loop() {
    with_runtime("testing-tick-intervals", |_| {
        let fired = std::rc::Rc::new(std::cell::Cell::new(0_u32));
        let count = std::rc::Rc::clone(&fired);
        crate::reactive::__test_set_clock(std::time::Instant::now());
        let _id = crate::reactive::interval(std::time::Duration::from_millis(10), move || {
            count.set(count.get() + 1);
        });
        assert_eq!(tick_intervals(), 0);
        crate::reactive::__test_advance_clock(std::time::Duration::from_millis(10));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(fired.get(), 1);
        crate::reactive::__test_clear_clock();
    });
}
