#![allow(
    dead_code,
    reason = "G4 write-seam helpers consumed by sibling modules in this DR-047 landing"
)]

use core::panic::Location;

use crate::reactive::runtime::{RuntimeState, with_rt};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DrawPhase {
    Idle,
    Effect,
    Layout,
    Paint,
}

pub(crate) const MAX_DRAIN_PASSES: u32 = 16;

pub(crate) type ReplayFn = Box<dyn FnOnce()>;

pub(crate) struct DeferredWrite {
    pub(crate) replay: ReplayFn,
    #[allow(
        dead_code,
        reason = "diagnostic-only field surfaced when drain cap is hit; consumed in future trace hook"
    )]
    pub(crate) caller: &'static Location<'static>,
}

#[track_caller]
#[inline]
pub(crate) fn assert_paint_safe(rt: &RuntimeState, site: &'static str) {
    debug_assert!(
        rt.phase != DrawPhase::Paint,
        "{site} called during DrawPhase::Paint — reactive writes are illegal in the paint pass \
         (call site: {})",
        Location::caller(),
    );
}

pub(crate) fn enqueue_deferred(replay: ReplayFn, caller: &'static Location<'static>) {
    with_rt(|rt| {
        rt.deferred_writes.push(DeferredWrite { replay, caller });
    });
}

#[allow(
    clippy::panic,
    reason = "DR-047 §8 mandates a panic on drain-loop divergence — the cap is a safety net for a runaway widget bug, not a recoverable error"
)]
pub(crate) fn drain_deferred_writes() {
    for _pass in 0..MAX_DRAIN_PASSES {
        let batch: Vec<DeferredWrite> = with_rt(|rt| core::mem::take(&mut rt.deferred_writes));
        if batch.is_empty() {
            return;
        }
        for write in batch {
            (write.replay)();
        }
    }
    panic!(
        "drain_deferred_writes exceeded MAX_DRAIN_PASSES ({MAX_DRAIN_PASSES}) — \
         a deferred-action handler repeatedly enqueues another deferred write"
    );
}
