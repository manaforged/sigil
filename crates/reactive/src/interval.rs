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
#[path = "interval_tests.rs"]
mod tests;
