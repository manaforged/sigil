use core::panic::Location;

use crate::reactive::runtime::RuntimeState;

kittest_pub! {
    /// Phase of the frame that the runtime is in.
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum DrawPhase {
        /// No frame is in progress; a frame starts and ends here.
        Idle,
        /// The frame runs its effects; it follows `Idle`.
        Effect,
        /// The frame lays out the tree; it follows `Effect`.
        Layout,
        /// The frame paints; it follows `Layout`, and a signal write here fails a debug assertion.
        Paint,
    }
}

#[track_caller]
#[inline]
pub(crate) fn assert_paint_safe(rt: &RuntimeState, site: &'static str) {
    debug_assert!(
        rt.phase != DrawPhase::Paint,
        "Signal::{site} called during DrawPhase::Paint — reactive writes are illegal in the paint pass \
         (call site: {})",
        Location::caller(),
    );
}
