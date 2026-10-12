use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::*;
use crate::prelude::{container, text};
use crate::reactive::signal;

#[test]
fn with_cx_and_with_runtime_run_the_closure_in_a_live_scope() {
    let mut ran = 0;
    with_cx("with-cx-signal", |_cx| {
        let s = signal(1);
        s.set(42);
        assert_eq!(s.get(), 42);
        ran += 1;
    });
    with_runtime("with-runtime-signal", |_| {
        let s = signal(7);
        s.set(99);
        assert_eq!(s.get(), 99);
        ran += 1;
    });
    assert_eq!(ran, 2);
}

#[test]
fn tree_size_counts_root_and_children() {
    assert_eq!(tree_size("tree-size-single", container()), 1);
    let tree = || container().child(text("a")).child(text("b"));
    assert_eq!(tree_size("tree-size-children", tree), 3);
}

#[test]
fn tick_intervals_fires_a_due_interval_in_a_hand_driven_loop() {
    with_runtime("testing-tick-intervals", |_| {
        let fired = Rc::new(Cell::new(0_u32));
        let count = Rc::clone(&fired);
        crate::reactive::__test_set_clock(Instant::now());
        let _id = crate::reactive::interval(Duration::from_millis(10), move || {
            count.set(count.get() + 1);
        });
        assert_eq!(tick_intervals(), 0);
        crate::reactive::__test_advance_clock(Duration::from_millis(10));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(fired.get(), 1);
        crate::reactive::__test_clear_clock();
    });
}
