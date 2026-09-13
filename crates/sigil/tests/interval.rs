#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use core::time::Duration;
use std::time::Instant;

use sigil::prelude::*;
use sigil::reactive::{
    __test_advance_clock, __test_clear_clock, __test_set_clock, Runtime, signal, tick_intervals,
};

#[test]
fn interval_drives_signal_via_synthetic_ticks() {
    Runtime::scope("interval::synthetic_ticks", || {
        __test_set_clock(Instant::now());

        let cx = Cx::new_root();
        let counter = signal(0_u32);
        cx.interval(Duration::from_millis(100), move || {
            counter.set(counter.get() + 1);
        });

        assert_eq!(tick_intervals(), 0, "no fires before the period elapses");
        assert_eq!(counter.get(), 0, "counter still zero before first fire");

        for i in 1..=5_u32 {
            __test_advance_clock(Duration::from_millis(100));
            let fired = tick_intervals();
            assert_eq!(fired, 1, "tick {i} fires exactly one interval");
            assert_eq!(
                counter.get(),
                i,
                "counter advances by one per tick (iteration {i})",
            );
        }

        assert_eq!(counter.get(), 5, "counter reflects 5 synthetic ticks");

        __test_clear_clock();
    });
}

#[test]
fn interval_cancels_when_scope_disposes() {
    use core::cell::Cell;
    use std::rc::Rc;

    __test_set_clock(Instant::now());
    let counter = Rc::new(Cell::new(0_u32));
    let writer = Rc::clone(&counter);

    Runtime::scope("interval::auto_cancel", || {
        let cx = Cx::new_root();
        cx.interval(Duration::from_millis(50), move || {
            writer.set(writer.get() + 1);
        });
        __test_advance_clock(Duration::from_millis(50));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(counter.get(), 1);
    });

    __test_advance_clock(Duration::from_millis(500));
    assert_eq!(tick_intervals(), 0, "no fires after owner disposed");
    assert_eq!(counter.get(), 1, "counter pinned at last in-scope fire");

    __test_clear_clock();
}
