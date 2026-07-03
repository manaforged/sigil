//! Frame-phase transitions on [`Runtime`].
//!
//! Extracted from `runtime.rs` to keep that file under the 400-line
//! cap. The `impl Runtime` block here exposes the four phase
//! transitions used by the application's frame driver:
//!
//! - [`Runtime::begin_effect`] — `Idle → Effect` plus the G5
//!   completion drain (DR-048 §4) and the prior frame's deferred
//!   writes.
//! - [`Runtime::begin_layout`] — `Effect → Layout` after observer
//!   drain to fixpoint.
//! - [`Runtime::begin_paint`] — `Layout → Paint` (no drain; the
//!   renderer's two-phase walker runs inside this phase).
//! - [`Runtime::end_frame`] — collapses `Paint → Effect → Idle`,
//!   replaying every deferred-write entry against the focus stack
//!   as it exists at replay time.

use crate::phase::{DrawPhase, drain_deferred_writes};
use crate::runtime::{Runtime, with_rt};
use crate::runtime_drain::{
    drain_a11y_updates, drain_observers_to_fixpoint, drain_pending_completions,
};

impl Runtime {
    /// Idle → Effect. Drain G5 completion callbacks (DR-048 §4), then
    /// the deferred writes captured during the prior frame's Paint
    /// pass, before observers run.
    ///
    /// # Panics
    ///
    /// Debug-panics if the current phase is not `Idle`.
    pub fn begin_effect() {
        with_rt(|rt| {
            debug_assert_eq!(
                rt.phase,
                DrawPhase::Idle,
                "Runtime::begin_effect called from phase {:?}, expected Idle",
                rt.phase,
            );
            rt.phase = DrawPhase::Effect;
            // New frame: clear the dirty flag so only mutations produced *this*
            // frame (by the drains below, by input drained in Effect, and by the
            // observer fixpoint in begin_layout) mark it again.
            rt.frame_dirty = false;
        });
        // G5 — drain task completions FIRST (DR-048 §4). Handlers may
        // entity.update / signal.set / cx.notify; phase is Effect so
        // those writes are legal. Re-entrant enqueue (a handler that
        // schedules another completion) lands in the same vec and
        // drains in the same loop.
        drain_pending_completions();
        // Drain deferred writes captured during the prior frame's
        // Paint pass; their handlers may write to the reactive graph
        // and that is legal now (phase is Effect).
        drain_deferred_writes();
    }

    /// Effect → Layout. Drain pending observer callbacks until the
    /// queue is stable (or `MAX_DRAIN_PASSES` is hit).
    ///
    /// The drain bounds protect against an oscillating observer cycle
    /// where each callback enqueues another observer notification.
    ///
    /// # Panics
    ///
    /// Debug-panics if the current phase is not `Effect`. Panics if
    /// observer drain does not converge within `MAX_DRAIN_PASSES`
    /// iterations.
    pub fn begin_layout() {
        with_rt(|rt| {
            debug_assert_eq!(
                rt.phase,
                DrawPhase::Effect,
                "Runtime::begin_layout called from phase {:?}, expected Effect",
                rt.phase,
            );
        });
        drain_observers_to_fixpoint();
        with_rt(|rt| rt.phase = DrawPhase::Layout);
    }

    /// Layout → Paint. The renderer's two-phase walker (DR-034) runs
    /// entirely inside this phase. Reactive writes are forbidden:
    /// direct mutations debug-panic; action dispatch defers.
    ///
    /// # Panics
    ///
    /// Debug-panics if the current phase is not `Layout`.
    pub fn begin_paint() {
        with_rt(|rt| {
            debug_assert_eq!(
                rt.phase,
                DrawPhase::Layout,
                "Runtime::begin_paint called from phase {:?}, expected Layout",
                rt.phase,
            );
            rt.phase = DrawPhase::Paint;
        });
    }

    /// Paint → Effect (drain deferred writes) → Idle.
    ///
    /// Internally collapses the three-step transition. Replays every
    /// deferred-write queue entry against the focus stack as it exists
    /// at replay time (DR-047 §5). Newly-enqueued writes during replay
    /// drain in the same pass via the `MAX_DRAIN_PASSES` cap.
    ///
    /// # Panics
    ///
    /// Debug-panics if the current phase is not `Paint`. Panics if
    /// the deferred-write drain exceeds `MAX_DRAIN_PASSES`.
    pub fn end_frame() {
        with_rt(|rt| {
            debug_assert_eq!(
                rt.phase,
                DrawPhase::Paint,
                "Runtime::end_frame called from phase {:?}, expected Paint",
                rt.phase,
            );
            // Transition to Effect so the replayed actions land in a
            // write-legal phase (the deferred dispatch path re-checks
            // phase and would otherwise loop into itself).
            rt.phase = DrawPhase::Effect;
        });
        drain_deferred_writes();
        // After replay, any observers scheduled by replayed actions
        // must also flush before the runtime returns to Idle, so the
        // application code that drives the next frame sees a clean
        // queue.
        drain_observers_to_fixpoint();
        // W5.2 / DR-050 §G10-phase-2 — once every signal mutation has
        // settled, snapshot the kernel tree through the installed
        // publisher and forward it to the bridge. Cheap no-op when no
        // observed signal changed this frame.
        drain_a11y_updates();
        with_rt(|rt| rt.phase = DrawPhase::Idle);
    }

    /// Whether a visual-affecting reactive mutation occurred since the current
    /// frame's [`Self::begin_effect`]. The frame driver reads this after
    /// `begin_layout` (the observer fixpoint has run by then) to decide whether
    /// layout + paint are needed, or whether the frame changed nothing (e.g. a
    /// bare cursor-move over inert UI) and the prior frame can stand.
    #[must_use]
    pub fn frame_dirty() -> bool {
        with_rt(|rt| rt.frame_dirty)
    }

    /// Force the current frame dirty from a non-reactive source — a structural
    /// change (first surface, resize) that the viewer must see but that writes
    /// no signal. Reactive mutations flag themselves; this is the escape hatch
    /// for the frame driver's own state changes.
    pub fn mark_frame_dirty() {
        with_rt(|rt| rt.frame_dirty = true);
    }
}
