use core::time::Duration;
use std::cell::RefCell;
use web_time::Instant;

use crate::reactive::owner::{Owner, OwnerId, on_cleanup};

use crate::reactive::interval_registry::{IntervalSlot, REGISTRY, invoke_by_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntervalId(pub(crate) u64);

#[track_caller]
pub fn interval(period: Duration, callback: impl Fn() + 'static) -> IntervalId {
    register(period, Some(period), Box::new(callback))
}

#[track_caller]
pub fn timeout(delay: Duration, callback: impl FnOnce() + 'static) -> IntervalId {
    let once = RefCell::new(Some(callback));
    let fire = move || {
        let taken = once.borrow_mut().take();
        if let Some(run) = taken {
            run();
        }
    };
    register(delay, None, Box::new(fire))
}

#[track_caller]
fn register(delay: Duration, period: Option<Duration>, callback: Box<dyn Fn()>) -> IntervalId {
    let owner = Owner::current().expect("interval or timeout called outside a reactive scope");
    let id = REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        let now = reg.now();
        let id = reg.allocate_id();
        reg.slots.push((
            id,
            IntervalSlot {
                owner,
                period,
                next_fire: now + delay,
                callback,
            },
        ));
        id
    });
    on_cleanup(move || cancel_interval(id));
    id
}

pub fn cancel_interval(id: IntervalId) {
    let removed = REGISTRY.try_with(|r| r.borrow_mut().remove(id));
    drop(removed);
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
            let mut reg = r.borrow_mut();
            let Some(slot) = reg.slot_mut(id) else {
                return;
            };
            match slot.period {
                Some(period) => slot.next_fire = now + period,
                None => drop(reg.remove(id)),
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
    REGISTRY.with(|r| r.borrow_mut().test_clock = Some(clock));
}

#[cfg(any(test, feature = "test-clock"))]
pub fn __test_advance_clock(delta: Duration) {
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        reg.test_clock = Some(reg.now() + delta);
    });
}

#[cfg(any(test, feature = "test-clock"))]
pub fn __test_clear_clock() {
    REGISTRY.with(|r| r.borrow_mut().test_clock = None);
}

#[cfg(test)]
#[path = "interval_tests.rs"]
mod tests;
