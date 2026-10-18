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
use crate::reactive::Runtime;
use crate::reactive::{effect_named, on_cleanup, signal};

fn counter() -> (Rc<Cell<u32>>, impl Fn() + 'static) {
    let count = Rc::new(Cell::new(0_u32));
    let writer = Rc::clone(&count);
    (count, move || writer.set(writer.get() + 1))
}

#[test]
fn interval_fires_after_period_elapses() {
    Runtime::scope("interval::fires_after_period", || {
        __test_set_clock(Instant::now());
        let (count, tick) = counter();
        let _id = interval(Duration::from_millis(100), tick);

        assert_eq!(tick_intervals(), 0);
        assert_eq!(count.get(), 0);

        __test_advance_clock(Duration::from_millis(100));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(count.get(), 1);

        __test_advance_clock(Duration::from_millis(300));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(count.get(), 2);

        __test_clear_clock();
    });
}

#[test]
fn interval_is_cancelled_when_owner_disposed() {
    __test_set_clock(Instant::now());
    let (count, tick) = counter();
    Runtime::scope("interval::cancel_on_dispose", || {
        interval(Duration::from_millis(50), tick);
    });
    __test_advance_clock(Duration::from_millis(500));
    assert_eq!(tick_intervals(), 0);
    assert_eq!(count.get(), 0);

    __test_clear_clock();
}

#[test]
fn next_interval_deadline_reports_soonest_slot() {
    Runtime::scope("interval::soonest", || {
        __test_set_clock(Instant::now());
        let _a = interval(Duration::from_millis(200), || {});
        let _b = interval(Duration::from_millis(50), || {});
        let deadline =
            next_interval_deadline().expect("two intervals registered, deadline must exist");
        let now = Instant::now();
        assert!(deadline >= now.checked_sub(Duration::from_millis(5)).unwrap());
        __test_clear_clock();
    });
}

#[test]
fn panic_in_callback_does_not_zombify_slot() {
    Runtime::scope("interval::panic_recovery", || {
        __test_set_clock(Instant::now());
        let hits = Rc::new(Cell::new(0_u32));
        let writer = Rc::clone(&hits);
        let _id = interval(Duration::from_millis(50), move || {
            let count = writer.get() + 1;
            writer.set(count);
            if count == 1 {
                panic!("first tick blows up");
            }
        });

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
        let (count, tick) = counter();
        let id = interval(Duration::from_millis(10), tick);
        cancel_interval(id);
        __test_advance_clock(Duration::from_millis(100));
        assert_eq!(tick_intervals(), 0);
        assert_eq!(count.get(), 0);
        __test_clear_clock();
    });
}

#[test]
fn interval_panic_drains_deferred_cleanups_and_clears_origin() {
    Runtime::scope("interval::panic_drain", || {
        __test_set_clock(Instant::now());
        let ran = Rc::new(Cell::new(false));
        let flag = Rc::clone(&ran);
        let ticks = Rc::new(Cell::new(0_u32));
        let sig_owner = Owner::new("sig", Owner::current());
        let mark = sig_owner.with(|| signal(0_u32));
        let fx_owner = Owner::new("fx", Owner::current());
        let _id = interval(Duration::from_millis(50), move || {
            if ticks.replace(1) != 0 {
                return;
            }
            sig_owner.dispose();
            let flag = Rc::clone(&flag);
            fx_owner.with(move || {
                effect_named("test::interval_reader", move || {
                    on_cleanup({
                        let flag = Rc::clone(&flag);
                        move || flag.set(true)
                    });
                    let _ = mark.get();
                });
            });
        });

        __test_advance_clock(Duration::from_millis(50));
        assert_eq!(tick_intervals(), 1);
        assert!(
            ran.get(),
            "cleanups deferred during the interval unwind must run at the catch site"
        );
        assert!(
            Runtime::panic_origin_info().is_none(),
            "the swallowed panic must not leave a stale origin behind"
        );

        __test_advance_clock(Duration::from_millis(50));
        assert_eq!(
            tick_intervals(),
            1,
            "the slot survives and ticks cleanly after recovery"
        );
        __test_clear_clock();
    });
}

struct CancelOnDrop(Rc<Cell<Option<IntervalId>>>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(id) = self.0.get() {
            cancel_interval(id);
        }
    }
}

#[test]
fn ending_a_scope_drops_an_interval_whose_callback_cancels_on_drop() {
    let worker = std::thread::spawn(|| {
        Runtime::scope("interval::cancel_on_drop", || {
            let slot = Rc::new(Cell::new(None));
            let guard = CancelOnDrop(Rc::clone(&slot));
            let id = interval(Duration::from_millis(50), move || {
                let _held = &guard;
            });
            slot.set(Some(id));
        });
    });
    assert!(worker.join().is_ok(), "ending the scope panicked");
}

#[test]
fn a_timeout_fires_once_and_then_stops_waking_the_loop() {
    Runtime::scope("interval::timeout_once", || {
        let start = Instant::now();
        __test_set_clock(start);
        let (count, tick) = counter();
        let _id = timeout(Duration::from_millis(10), tick);
        assert_eq!(
            next_interval_deadline(),
            Some(start + Duration::from_millis(10))
        );
        __test_advance_clock(Duration::from_millis(10));
        assert_eq!(tick_intervals(), 1);
        assert_eq!(count.get(), 1);
        assert_eq!(next_interval_deadline(), None);
        __test_advance_clock(Duration::from_millis(100));
        assert_eq!(tick_intervals(), 0);
        assert_eq!(count.get(), 1);
        __test_clear_clock();
    });
}
