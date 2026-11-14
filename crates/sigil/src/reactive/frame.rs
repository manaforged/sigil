use core::cell::{Cell, RefCell};
use std::rc::Rc;

use slotmap::{SlotMap, new_key_type};
use web_time::Instant;

use super::interval::{Deadline, tick_intervals};
use crate::animation::runtime::clock::{ClockSource, tick_clocks};
use crate::animation::runtime::state::AnimatorSource;
use crate::animation::runtime::{animating, tick};

new_key_type! {
    pub(crate) struct SourceId;
}

pub(crate) enum FrameSource {
    Deadline(Deadline),
    Animator(AnimatorSource),
    Clock(ClockSource),
    Callback(Rc<RefCell<dyn FnMut(f32)>>),
}

pub(crate) struct Sources {
    pub(crate) map: SlotMap<SourceId, FrameSource>,
    next_seq: u64,
}

impl Sources {
    fn new() -> Self {
        Self {
            map: SlotMap::with_key(),
            next_seq: 0,
        }
    }

    pub(crate) const fn next_seq(&mut self) -> u64 {
        self.next_seq = self.next_seq.wrapping_add(1);
        self.next_seq
    }
}

thread_local! {
    static SOURCES: RefCell<Sources> = RefCell::new(Sources::new());
}

pub(crate) fn with_sources<R>(change: impl FnOnce(&mut Sources) -> R) -> R {
    SOURCES.with(|cell| change(&mut cell.borrow_mut()))
}

thread_local! {
    static TEST_CLOCK: Cell<Option<Instant>> = const { Cell::new(None) };
}

pub(crate) fn now() -> Instant {
    TEST_CLOCK
        .try_with(Cell::get)
        .ok()
        .flatten()
        .unwrap_or_else(Instant::now)
}

#[cfg(feature = "test-clock")]
pub(crate) fn set_test_clock(clock: Option<Instant>) {
    let _ = TEST_CLOCK.try_with(|slot| slot.set(clock));
}

pub(crate) fn try_with_sources<R>(change: impl FnOnce(&mut Sources) -> R) -> Option<R> {
    SOURCES.try_with(|cell| change(&mut cell.borrow_mut())).ok()
}

#[must_use = "the frame loop reports interval fires and whether motion continues"]
pub(crate) fn tick_frame(dt: f32) -> (usize, bool) {
    let intervals = tick_intervals();
    if animating() {
        tick(dt);
    }
    tick_clocks(dt);
    (intervals, animating())
}
