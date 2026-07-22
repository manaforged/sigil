//! Phase F1 — `Cx::interval` integration test.
//!
//! Drives 5 synthetic 100 ms ticks through the reactive interval
//! registry against a pinned test clock (no wall-clock sleeps) and
//! asserts the counter signal advanced by 5.
//!
//! The test exercises the full registration + scheduling path:
//!
//! 1. `Cx::interval` registers a callback in the reactive interval
//!    registry, owned by the current reactive scope.
//! 2. `__test_set_clock` pins the registry's view of "now" so we can
//!    advance time deterministically.
//! 3. `__test_advance_clock(100 ms)` + `tick_intervals()` per loop
//!    iteration fires the due interval and rebases its next-fire time.
//! 4. The callback mutates a `Signal<u32>` — proving signal writes
//!    propagate from inside an interval, the property the dogfood-tier-1
//!    timer-stopwatch app needs for its rolling display.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

// Dev-dep aliases — the lib-test target pulls these in via sigil's
// dev-dep set but this file doesn't name them directly.
use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use core::time::Duration;
use std::time::Instant;

use sigil::prelude::*;
use sigil_reactive::{
    __test_advance_clock, __test_clear_clock, __test_set_clock, Runtime, signal, tick_intervals,
};

#[test]
fn interval_drives_signal_via_synthetic_ticks() {
    Runtime::scope("interval::synthetic_ticks", || {
        // Pin "now" so the test is deterministic (no wall-clock).
        __test_set_clock(Instant::now());

        let cx = Cx::new_root();
        let counter = signal(0_u32);
        cx.interval(Duration::from_millis(100), move || {
            counter.set(counter.get() + 1);
        });

        // No time elapsed yet → no fire.
        assert_eq!(tick_intervals(), 0, "no fires before the period elapses");
        assert_eq!(counter.get(), 0, "counter still zero before first fire");

        // Drive 5 ticks. Each advance + tick is one fire because we
        // rebase next_fire to (now + period) per fire.
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
    // Use `Rc<Cell<u32>>` instead of `signal` because the counter has
    // to live across the scope boundary; signals require an enclosing
    // reactive scope at construction time.
    let counter = Rc::new(Cell::new(0_u32));
    let writer = Rc::clone(&counter);

    Runtime::scope("interval::auto_cancel", || {
        let cx = Cx::new_root();
        cx.interval(Duration::from_millis(50), move || {
            writer.set(writer.get() + 1);
        });
        // First fire inside the scope so we know registration worked.
        __test_advance_clock(Duration::from_millis(50));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(counter.get(), 1);
    });

    // Scope disposed — cleanup ran, slot evicted. Further ticks no-op.
    __test_advance_clock(Duration::from_millis(500));
    assert_eq!(tick_intervals(), 0, "no fires after owner disposed");
    assert_eq!(counter.get(), 1, "counter pinned at last in-scope fire");

    __test_clear_clock();
}
