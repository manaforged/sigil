use core::panic::Location;

use crate::reactive::runtime::RuntimeState;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DrawPhase {
    Idle,
    Effect,
    Layout,
    Paint,
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
