use core::cell::RefCell;
use slotmap::{SlotMap, new_key_type};
use web_time::Instant;

use super::interval::{Deadline, tick_intervals};
use crate::animation_runtime::clock::{ClockSource, running_clocks, tick_clocks};
use crate::animation_runtime::state::AnimatorSource;
use crate::animation_runtime::{active_count, tick};

new_key_type! {
    pub(crate) struct SourceId;
}

pub(crate) enum FrameSource {
    Deadline(Deadline),
    Animator(AnimatorSource),
    Clock(ClockSource),
}

pub(crate) struct Sources {
    pub(crate) map: SlotMap<SourceId, FrameSource>,
    next_seq: u64,
    pub(crate) test_clock: Option<Instant>,
}

impl Sources {
    fn new() -> Self {
        Self {
            map: SlotMap::with_key(),
            next_seq: 0,
            test_clock: None,
        }
    }

    pub(crate) fn now(&self) -> Instant {
        self.test_clock.unwrap_or_else(Instant::now)
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

pub(crate) fn try_with_sources<R>(change: impl FnOnce(&mut Sources) -> R) -> Option<R> {
    SOURCES.try_with(|cell| change(&mut cell.borrow_mut())).ok()
}

#[must_use = "the frame loop reports interval fires and whether motion continues"]
pub(crate) fn tick_frame(dt: f32) -> (usize, bool) {
    let intervals = tick_intervals();
    let animating = active_count() > 0;
    if animating || running_clocks() > 0 {
        tick(dt);
    }
    tick_clocks(dt);
    (intervals, animating || running_clocks() > 0)
}
