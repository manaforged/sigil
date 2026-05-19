//! G4 `DrawPhase` — frame-phase state machine + write-during-Paint
//! assertion seam (DR-047).
//!
//! Holds the [`DrawPhase`] enum that lives on [`crate::runtime::RuntimeState`]
//! plus the [`assert_paint_safe`] helper every reactive write seam
//! consults, the deferred-write queue that captures Paint-phase
//! [`crate::cx::Context::dispatch_action`] calls, and the drain that
//! replays them at the top of the next Effect phase.
//!
//! See DR-047 for the contract and `docs/specs/g4-drawphase-api-sketch.md`
//! for the API sketch this module realises.

// The write-seam helpers (`assert_paint_safe`, `enqueue_deferred`) are
// wired into `signal_write.rs`, `entity.rs`, `cx.rs`, `entity_global.rs`
// in this same G4 landing. During incremental compilation between
// edits the dead-code lint fires before all consumers exist — scoped
// allow keeps the module reviewable as one unit.
#![allow(
    dead_code,
    reason = "G4 write-seam helpers consumed by sibling modules in this DR-047 landing"
)]

use core::panic::Location;

use crate::runtime::{RuntimeState, with_rt};

/// Frame phase for the reactive runtime.
///
/// One frame transitions Idle → Effect → Layout → Paint → Idle.
/// The phase is consulted by every reactive write path; writes
/// during [`DrawPhase::Paint`] are illegal in debug (panic) and
/// either no-op or deferred in release (see DR-047 §5).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DrawPhase {
    /// Resting phase between frames. Reactive writes are allowed.
    /// This is the runtime's default phase at startup and after
    /// [`crate::Runtime::end_frame`] returns.
    Idle,
    /// Effect fan-out phase. Observers fire, action handlers
    /// dispatch, effects re-run. Writes allowed.
    Effect,
    /// Layout solve phase. Layout reads but does not write by
    /// convention; reactive writes are still allowed (Layout is not
    /// Paint), but mutating during layout invalidates the layout
    /// pass.
    Layout,
    /// Paint pass — renderer walks the styled tree emitting GPU
    /// commands. Reactive writes are illegal: direct mutations panic
    /// in debug and no-op in release; [`crate::cx::Context::dispatch_action`]
    /// defers to the next Effect phase.
    Paint,
}

/// Maximum drain passes for the deferred-write + observer drain loop
/// inside [`crate::Runtime::end_frame`].
///
/// Catches a runaway cycle where a replayed action dispatches another
/// action which dispatches another action ad infinitum. In normal use
/// 1-3 passes is the expected count; 16 is a conservative ceiling.
pub(crate) const MAX_DRAIN_PASSES: u32 = 16;

/// Boxed deferred-action replay closure.
///
/// Captures the cloned action plus the dispatch path. When called
/// during [`drain_deferred_writes`], the closure re-enters
/// [`crate::cx::Context::dispatch_action`] against the focus stack as
/// it exists at replay time (DR-047 §5).
pub(crate) type ReplayFn = Box<dyn FnOnce()>;

/// Single entry in the deferred-write queue.
///
/// `caller` is the `#[track_caller]`-captured location of the original
/// `dispatch_action` call site, preserved across the defer/replay
/// boundary for diagnostics.
pub(crate) struct DeferredWrite {
    pub(crate) replay: ReplayFn,
    #[allow(
        dead_code,
        reason = "diagnostic-only field surfaced when drain cap is hit; consumed in future trace hook"
    )]
    pub(crate) caller: &'static Location<'static>,
}

/// Panic in debug, no-op in release, if the runtime is currently in
/// [`DrawPhase::Paint`].
///
/// Called from every reactive write seam (`Signal::set`, `Signal::update`,
/// `Entity::update` / `run_update`, `Context::notify`, `Context::emit`,
/// `Context::focus`, `Context::blur`, `set_global`). The
/// [`crate::cx::Context::dispatch_action`] path takes a different
/// branch (it defers in release rather than no-op) via
/// [`enqueue_deferred`].
///
/// `#[track_caller]` ensures the panic location is the user's call
/// site, not this helper.
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

/// Enqueue a deferred replay closure.
///
/// Called from [`crate::cx::Context::dispatch_action`] when the
/// runtime is in [`DrawPhase::Paint`] and (in release) the
/// debug-assert was compiled out. The closure executes at the top of
/// [`crate::Runtime::end_frame`] / [`drain_deferred_writes`] under the
/// then-current focus stack.
pub(crate) fn enqueue_deferred(replay: ReplayFn, caller: &'static Location<'static>) {
    with_rt(|rt| {
        rt.deferred_writes.push(DeferredWrite { replay, caller });
    });
}

/// Drain `deferred_writes`, replaying each entry in arrival order.
///
/// Re-enqueues during replay land in the same queue and are drained
/// in the same pass. The fixed `MAX_DRAIN_PASSES` cap prevents a
/// runaway cycle (a replayed handler that always dispatches another
/// deferred action).
///
/// # Panics
///
/// Panics if the drain does not converge within `MAX_DRAIN_PASSES`
/// iterations — surface for a buggy widget that hot-loops on
/// `dispatch_action` from inside its paint code. The panic message is
/// the only actionable signal for that bug class, which is why this
/// function carries an explicit `clippy::panic` allow: the cap is a
/// safety net, not a recoverable error path.
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
