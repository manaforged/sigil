use core::time::Duration;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use web_time::Instant;

use crate::reactive::owner::{OwnerId, owner_by_id};
use crate::reactive::runtime::Runtime;

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
        Runtime::recover_after_swallowed_panic();
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

#[cfg(any(test, feature = "kittest"))]
pub(super) fn test_set_clock(clock: Instant) {
    REGISTRY.with(|r| r.borrow_mut().test_clock = Some(clock));
}

#[cfg(any(test, feature = "kittest"))]
pub(super) fn test_advance_clock(delta: Duration) {
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        let base = reg.test_clock.unwrap_or_else(Instant::now);
        reg.test_clock = Some(base + delta);
    });
}

#[cfg(any(test, feature = "kittest"))]
pub(super) fn test_clear_clock() {
    REGISTRY.with(|r| r.borrow_mut().test_clock = None);
}
