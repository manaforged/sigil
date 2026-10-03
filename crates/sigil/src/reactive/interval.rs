use core::time::Duration;
use web_time::Instant;

use crate::reactive::owner::{Owner, OwnerId, on_cleanup};

use crate::reactive::interval_registry::{IntervalSlot, REGISTRY, invoke_by_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalId(pub(crate) u64);

#[track_caller]
pub fn interval(period: Duration, callback: impl Fn() + 'static) -> IntervalId {
    let owner = Owner::current().expect("interval called outside a reactive scope");
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

pub fn cancel_interval(id: IntervalId) {
    REGISTRY.with(|r| r.borrow_mut().remove(id));
}

#[must_use = "shells can ignore the count; tests use it to assert that fires actually happened"]
pub(crate) fn tick_intervals() -> usize {
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

#[cfg(any(test, feature = "test-clock"))]
pub fn __test_set_clock(clock: Instant) {
    crate::reactive::interval_registry::test_set_clock(clock);
}

#[cfg(any(test, feature = "test-clock"))]
pub fn __test_advance_clock(delta: Duration) {
    crate::reactive::interval_registry::test_advance_clock(delta);
}

#[cfg(any(test, feature = "test-clock"))]
pub fn __test_clear_clock() {
    crate::reactive::interval_registry::test_clear_clock();
}

#[cfg(test)]
#[path = "interval_tests.rs"]
mod tests;
