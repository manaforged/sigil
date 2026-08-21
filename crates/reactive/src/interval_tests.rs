#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::manual_assert,
    clippy::shadow_unrelated
)]

use core::cell::Cell;
use std::rc::Rc;

use super::*;
use crate::Runtime;

#[test]
fn interval_fires_after_period_elapses() {
    Runtime::scope("interval::fires_after_period", || {
        __test_set_clock(Instant::now());
        let count = Rc::new(Cell::new(0_u32));
        let writer = Rc::clone(&count);
        let _id = register_interval(Duration::from_millis(100), move || {
            writer.set(writer.get() + 1);
        });

        // No time has passed — no fire.
        assert_eq!(tick_intervals(), 0);
        assert_eq!(count.get(), 0);

        // Advance 100 ms — one fire.
        __test_advance_clock(Duration::from_millis(100));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(count.get(), 1);

        // Advance 300 ms — only one fire because tick rebases
        // next_fire to now + period on each tick.
        __test_advance_clock(Duration::from_millis(300));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(count.get(), 2);

        __test_clear_clock();
    });
}

#[test]
fn interval_is_cancelled_when_owner_disposed() {
    __test_set_clock(Instant::now());
    let count = Rc::new(Cell::new(0_u32));
    let writer = Rc::clone(&count);

    Runtime::scope("interval::cancel_on_dispose", || {
        register_interval(Duration::from_millis(50), move || {
            writer.set(writer.get() + 1);
        });
    });
    // Owner disposed — slot evicted. No fires after advance.
    __test_advance_clock(Duration::from_millis(500));
    assert_eq!(tick_intervals(), 0);
    assert_eq!(count.get(), 0);

    __test_clear_clock();
}

#[test]
fn next_interval_deadline_reports_soonest_slot() {
    Runtime::scope("interval::soonest", || {
        __test_set_clock(Instant::now());
        let _a = register_interval(Duration::from_millis(200), || {});
        let _b = register_interval(Duration::from_millis(50), || {});
        let deadline =
            next_interval_deadline().expect("two intervals registered, deadline must exist");
        let now = Instant::now();
        // Deadline is ~50 ms from the test clock (which we set to
        // a real `Instant::now()` snapshot); within tolerance.
        assert!(deadline >= now.checked_sub(Duration::from_millis(5)).unwrap());
        __test_clear_clock();
    });
}

#[test]
fn panic_in_callback_does_not_zombify_slot() {
    // A panicking user callback must be caught — the slot must
    // be restored with its original callback so a subsequent tick
    // re-runs the same logic instead of silently calling the
    // no-op placeholder.
    Runtime::scope("interval::panic_recovery", || {
        __test_set_clock(Instant::now());
        let hits = Rc::new(Cell::new(0_u32));
        let writer = Rc::clone(&hits);
        let _id = register_interval(Duration::from_millis(50), move || {
            let count = writer.get() + 1;
            writer.set(count);
            if count == 1 {
                panic!("first tick blows up");
            }
        });

        // First tick — callback panics, but invoke_by_id catches
        // it and returns true. The slot's callback is restored.
        __test_advance_clock(Duration::from_millis(50));
        let fired = tick_intervals();
        assert_eq!(
            fired, 1,
            "panicking callback still counts as fired (it ran, just panicked)"
        );
        assert_eq!(
            hits.get(),
            1,
            "callback observed the panic-causing first tick"
        );

        // Second tick — the original callback (still installed in
        // the slot, NOT the no-op stub) must run again. This is
        // the regression: previously the slot was zombified with
        // the stub and this assertion would fail at 1.
        __test_advance_clock(Duration::from_millis(50));
        let fired = tick_intervals();
        assert_eq!(fired, 1, "slot must still be live after the prior panic");
        assert_eq!(hits.get(), 2, "callback re-ran cleanly on the second tick");

        __test_clear_clock();
    });
}

#[test]
fn cancel_interval_removes_slot_immediately() {
    Runtime::scope("interval::cancel_explicit", || {
        __test_set_clock(Instant::now());
        let count = Rc::new(Cell::new(0_u32));
        let writer = Rc::clone(&count);
        let id = register_interval(Duration::from_millis(10), move || {
            writer.set(writer.get() + 1);
        });
        cancel_interval(id);
        __test_advance_clock(Duration::from_millis(100));
        assert_eq!(tick_intervals(), 0);
        assert_eq!(count.get(), 0);
        __test_clear_clock();
    });
}
