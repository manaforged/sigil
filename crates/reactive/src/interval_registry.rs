//! Internal plumbing for the interval primitive.
//!
//! Holds the thread-local slot registry, the test-clock seam, and the
//! panic-safe invocation helper. Public API lives in
//! [`super`](crate::interval); this module is the implementation side
//! and exposes only what `interval.rs` needs via `pub(super)`.

use core::time::Duration;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use crate::owner::{OwnerId, owner_by_id};

use super::IntervalId;

pub(super) struct IntervalSlot {
    pub(super) owner: OwnerId,
    pub(super) period: Duration,
    pub(super) next_fire: Instant,
    pub(super) callback: Box<dyn Fn()>,
}

pub(super) struct Registry {
    next_id: u64,
    pub(super) slots: Vec<(IntervalId, IntervalSlot)>,
    test_clock: Option<Instant>,
}

impl Registry {
    const fn new() -> Self {
        Self {
            next_id: 1,
            slots: Vec::new(),
            test_clock: None,
        }
    }

    pub(super) const fn allocate_id(&mut self) -> IntervalId {
        let id = IntervalId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    pub(super) fn now(&self) -> Instant {
        self.test_clock.unwrap_or_else(Instant::now)
    }

    pub(super) fn remove(&mut self, id: IntervalId) {
        self.slots.retain(|(slot_id, _)| *slot_id != id);
    }
}

thread_local! {
    pub(super) static REGISTRY: RefCell<Registry> = const { RefCell::new(Registry::new()) };
}

/// Invoke the stored callback for `id` inside `owner`'s scope.
///
/// Returns `true` when the callback ran. The callback is swapped out
/// of the registry while it runs so callbacks can re-enter
/// [`register_interval`](super::register_interval) /
/// [`cancel_interval`](super::cancel_interval) without `RefCell` panics.
///
/// # Panic safety
///
/// User callbacks run inside [`std::panic::catch_unwind`]. If a
/// callback panics:
///
/// * the panic is captured and a one-line diagnostic is written to
///   `stderr` with the slot id, so the issue is surfaced without
///   crashing the shell;
/// * the original callback is restored to its slot, so the next tick
///   will try it again (slots are NOT zombified with the no-op stub);
/// * the tick returns `true` for the panicked slot — the callback ran,
///   even though it exited abnormally — so per-tick fire-count
///   reporting stays meaningful.
///
/// Repeatedly-panicking callbacks will repeatedly log; cancel them
/// explicitly via [`cancel_interval`](super::cancel_interval) when that
/// is undesirable.
pub(super) fn invoke_by_id(id: IntervalId, owner: OwnerId) -> bool {
    let cb = REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.slots
            .iter_mut()
            .find(|(slot_id, _)| *slot_id == id)
            .map(|slot| {
                let stub: Box<dyn Fn()> = Box::new(|| {});
                core::mem::replace(&mut slot.1.callback, stub)
            })
    });
    let Some(cb) = cb else {
        return false;
    };
    let cb_ref = &*cb;
    let result = catch_unwind(AssertUnwindSafe(|| owner_by_id(owner, cb_ref)));
    let ran = result.unwrap_or_else(|_| {
        eprintln!(
            "[sigil-reactive] interval callback panicked (id={}); \
             slot restored, callback will re-run on next tick",
            id.0
        );
        true
    });
    REGISTRY.with(|r| {
        if let Some(slot) = r
            .borrow_mut()
            .slots
            .iter_mut()
            .find(|(slot_id, _)| *slot_id == id)
        {
            slot.1.callback = cb;
        }
    });
    ran
}

/// Test-only: pin the registry's view of "now" to `clock`.
pub(super) fn test_set_clock(clock: Instant) {
    REGISTRY.with(|r| r.borrow_mut().test_clock = Some(clock));
}

/// Test-only: advance the pinned test clock by `delta`.
///
/// Initialises the pinned clock to `Instant::now()` if none was set.
pub(super) fn test_advance_clock(delta: Duration) {
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        let base = reg.test_clock.unwrap_or_else(Instant::now);
        reg.test_clock = Some(base + delta);
    });
}

/// Test-only: clear the pinned test clock.
pub(super) fn test_clear_clock() {
    REGISTRY.with(|r| r.borrow_mut().test_clock = None);
}
