//! Interval primitive — periodic callbacks driven by the run loop.
//!
//! An *interval* is a callback that fires every `period` while its
//! owning reactive scope is alive. Intervals are how widgets drive
//! real-time displays (clock, stopwatch, progress) from inside the
//! reactive graph.
//!
//! # Lifecycle
//!
//! [`register_interval`] inserts a slot into a thread-local registry
//! and registers an [`on_cleanup`](crate::on_cleanup) against the
//! current reactive owner. When the owner is disposed the slot is
//! evicted automatically — apps never need to track interval handles
//! manually.
//!
//! # Driving ticks
//!
//! The shell (or a test harness) drives interval firing by:
//!
//! 1. Calling [`tick_intervals`] once per frame. Every slot whose
//!    `next_fire <= now` runs its callback inside the registering
//!    owner's scope (so signal writes propagate cleanly), then has
//!    its `next_fire` rescheduled to `now + period`.
//! 2. Optionally calling [`next_interval_deadline`] to ask "when is
//!    the soonest interval due?" — used by the winit shell to set
//!    `ControlFlow::WaitUntil` instead of busy-looping.
//!
//! # Test clock
//!
//! Real-time integration tests would either sleep (slow + flaky) or
//! mock the clock. The [`__test_set_clock`] / [`__test_advance_clock`]
//! seams let integration tests advance the registry's view of "now"
//! deterministically without wall-clock sleeps. These are
//! `#[doc(hidden)]` — not part of the supported public API.

use core::time::Duration;
use std::time::Instant;

use crate::owner::{Owner, OwnerId, on_cleanup};

use crate::interval_registry::{self, IntervalSlot, REGISTRY, invoke_by_id};

/// Opaque, stable identifier for a registered interval.
///
/// Returned by [`register_interval`]. Holding an `IntervalId` is not
/// required for cleanup — the interval is evicted automatically when
/// its owning reactive scope is disposed — but callers may use the
/// id with [`cancel_interval`] for explicit early cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// `pub(crate)` on the field (not `pub`) so the sibling
// `interval_registry` module can allocate ids and log the raw value on
// panic without leaking the inner repr to crate consumers. Externally
// the type stays opaque.
pub struct IntervalId(pub(crate) u64);

/// Register a periodic callback owned by the current reactive scope.
///
/// The callback fires every `period` once [`tick_intervals`] is called
/// at-or-after the next fire time. The first fire occurs at
/// `registration_time + period` — intervals do not fire immediately on
/// registration.
///
/// When the current reactive owner is disposed, the interval is
/// automatically cancelled via [`on_cleanup`](crate::on_cleanup). No
/// manual teardown is required.
///
/// # Panics
///
/// Panics if called outside any reactive scope (mirroring
/// [`effect`](crate::effect)).
#[track_caller]
pub fn register_interval(period: Duration, callback: impl Fn() + 'static) -> IntervalId {
    let owner = Owner::current().expect("register_interval called outside a reactive scope");
    let now = REGISTRY.with(|r| r.borrow().now());
    let id = REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        let id = reg.allocate_id();
        reg.slots.push((
            id,
            IntervalSlot {
                owner,
                period,
                next_fire: now + period,
                callback: Box::new(callback),
            },
        ));
        id
    });
    on_cleanup(move || cancel_interval(id));
    id
}

/// Explicitly cancel an interval before its owning scope is disposed.
///
/// No-op if the id is stale (already cancelled or its owner already
/// disposed).
pub fn cancel_interval(id: IntervalId) {
    REGISTRY.with(|r| r.borrow_mut().remove(id));
}

/// Fire every interval whose `next_fire <= now`, then reschedule them.
///
/// Returns the number of callbacks that fired. Each callback runs
/// inside the scope of the owner that registered the interval, so
/// signal writes propagate to subscribers as if the registering code
/// had run them directly.
///
/// Callbacks whose owner has already been disposed between the cleanup
/// registration and this tick are silently dropped — the cleanup path
/// removes the slot first, so this is just a safety net.
///
/// Re-entrant: it is safe for a callback to register or cancel further
/// intervals. The registry borrow is released before any callback runs;
/// the callback is swapped out with a no-op placeholder for the
/// duration of its invocation and restored after.
///
/// # Rescheduling — "rebase to now + period", missed ticks are NOT replayed
///
/// On every tick the rescheduler sets `next_fire = now + period` (not
/// `previous_next_fire + period`). If the shell pauses for several
/// periods between calls — frame drop, suspended event loop,
/// long-running blocking work — the interval fires **once** when the
/// shell resumes, not once per missed period. This matches the
/// `setInterval` semantics most web devs already expect and avoids
/// burst-firing after a long pause.
///
/// If catch-up replay is required for a specific use case (animation
/// frames, fixed-step simulation), drive that subsystem from the
/// shell's frame loop directly rather than through `register_interval`.
#[must_use = "shells can ignore the count; tests use it to assert that fires actually happened"]
pub fn tick_intervals() -> usize {
    let now = REGISTRY.with(|r| r.borrow().now());
    let due: Vec<(IntervalId, OwnerId)> = REGISTRY.with(|r| {
        r.borrow()
            .slots
            .iter()
            .filter(|(_, slot)| slot.next_fire <= now)
            .map(|(id, slot)| (*id, slot.owner))
            .collect()
    });
    let mut fired = 0;
    for (id, owner) in due {
        if invoke_by_id(id, owner) {
            fired += 1;
        }
        REGISTRY.with(|r| {
            if let Some(slot) = r
                .borrow_mut()
                .slots
                .iter_mut()
                .find(|(slot_id, _)| *slot_id == id)
            {
                slot.1.next_fire = now + slot.1.period;
            }
        });
    }
    fired
}

/// Return the soonest `next_fire` across every registered interval.
///
/// `None` when no intervals are registered. Shells use this to set
/// `EventLoop::ControlFlow::WaitUntil(deadline)` so the event loop
/// idles between ticks instead of busy-waiting.
#[must_use]
pub fn next_interval_deadline() -> Option<Instant> {
    REGISTRY.with(|r| {
        r.borrow()
            .slots
            .iter()
            .map(|(_, slot)| slot.next_fire)
            .min()
    })
}

/// Test-only: pin the registry's view of "now" to `clock`.
///
/// While set, [`register_interval`], [`tick_intervals`], and
/// [`next_interval_deadline`] consult `clock` instead of
/// [`Instant::now`]. Call [`__test_clear_clock`] to restore wall-clock
/// behaviour.
///
/// Not part of the supported public API.
#[doc(hidden)]
pub fn __test_set_clock(clock: Instant) {
    interval_registry::test_set_clock(clock);
}

/// Test-only: advance the pinned test clock by `delta`.
///
/// Initialises the pinned clock to `Instant::now()` if none was set.
/// Not part of the supported public API.
#[doc(hidden)]
pub fn __test_advance_clock(delta: Duration) {
    interval_registry::test_advance_clock(delta);
}

/// Test-only: clear the pinned test clock.
///
/// Not part of the supported public API.
#[doc(hidden)]
pub fn __test_clear_clock() {
    interval_registry::test_clear_clock();
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::manual_assert,
    clippy::shadow_unrelated
)]
mod tests {
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
}
