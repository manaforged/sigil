#![allow(clippy::unwrap_used, clippy::expect_used)]

use core::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use crate::reactive::{
    __test_advance_clock, __test_clear_clock, __test_set_clock, Runtime, tick_intervals,
};

use super::*;

#[test]
fn cx_interval_fires_via_tick() {
    Runtime::scope("cx_interval::fires", || {
        __test_set_clock(Instant::now());
        let cx = Cx::new_root();
        let count = Rc::new(Cell::new(0_u32));
        let writer = Rc::clone(&count);
        cx.interval(Duration::from_millis(100), move || {
            writer.set(writer.get() + 1);
        });

        for _ in 0..3 {
            __test_advance_clock(Duration::from_millis(100));
            let _ = tick_intervals();
        }
        assert_eq!(count.get(), 3);
        __test_clear_clock();
    });
}
