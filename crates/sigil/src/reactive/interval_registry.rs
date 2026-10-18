use core::time::Duration;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use web_time::Instant;

use crate::reactive::owner::{OwnerId, owner_by_id};
use crate::reactive::runtime::Runtime;

use super::IntervalId;

pub(super) struct IntervalSlot {
    pub(super) owner: OwnerId,
    pub(super) period: Option<Duration>,
    pub(super) next_fire: Instant,
    pub(super) callback: Box<dyn Fn()>,
}

pub(super) struct Registry {
    next_id: u64,
    pub(super) slots: Vec<(IntervalId, IntervalSlot)>,
    pub(super) test_clock: Option<Instant>,
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

    pub(super) fn slot_mut(&mut self, id: IntervalId) -> Option<&mut IntervalSlot> {
        self.slots
            .iter_mut()
            .find(|(slot_id, _)| *slot_id == id)
            .map(|(_, slot)| slot)
    }

    pub(super) fn remove(&mut self, id: IntervalId) -> Vec<(IntervalId, IntervalSlot)> {
        let (gone, kept) = core::mem::take(&mut self.slots)
            .into_iter()
            .partition(|(slot_id, _)| *slot_id == id);
        self.slots = kept;
        gone
    }
}

thread_local! {
    pub(super) static REGISTRY: RefCell<Registry> = const { RefCell::new(Registry::new()) };
}

pub(super) fn invoke_by_id(id: IntervalId, owner: OwnerId) -> bool {
    let Some(cb) = REGISTRY.with(|r| {
        let stub: Box<dyn Fn()> = Box::new(|| {});
        r.borrow_mut()
            .slot_mut(id)
            .map(|slot| core::mem::replace(&mut slot.callback, stub))
    }) else {
        return false;
    };
    let ran = catch_unwind(AssertUnwindSafe(|| owner_by_id(owner, &*cb))).unwrap_or_else(|_| {
        eprintln!(
            "[sigil-reactive] interval callback panicked (id={}); \
             slot restored, callback will re-run on next tick",
            id.0
        );
        Runtime::recover_after_swallowed_panic();
        true
    });
    REGISTRY.with(|r| {
        if let Some(slot) = r.borrow_mut().slot_mut(id) {
            slot.callback = cb;
        }
    });
    ran
}
