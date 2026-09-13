use crate::reactive::phase::{DrawPhase, drain_deferred_writes};
use crate::reactive::runtime::{Runtime, with_rt};
use crate::reactive::runtime_drain::{drain_observers_to_fixpoint, drain_pending_completions};

impl Runtime {
    pub fn recover_from_panic() {
        with_rt(|rt| {
            rt.phase = DrawPhase::Idle;
            rt.running.clear();
            rt.pending.clear();
            rt.pending_set.clear();
            rt.frame_dirty = false;
        });
    }

    pub fn begin_effect() {
        with_rt(|rt| {
            debug_assert_eq!(
                rt.phase,
                DrawPhase::Idle,
                "Runtime::begin_effect called from phase {:?}, expected Idle",
                rt.phase,
            );
            rt.phase = DrawPhase::Effect;
            rt.frame_dirty = false;
        });
        drain_pending_completions();
        drain_deferred_writes();
    }

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

    pub fn end_frame() {
        with_rt(|rt| {
            debug_assert_eq!(
                rt.phase,
                DrawPhase::Paint,
                "Runtime::end_frame called from phase {:?}, expected Paint",
                rt.phase,
            );
            rt.phase = DrawPhase::Effect;
        });
        drain_deferred_writes();
        drain_observers_to_fixpoint();
        with_rt(|rt| rt.phase = DrawPhase::Idle);
    }

    #[must_use]
    pub fn frame_dirty() -> bool {
        with_rt(|rt| rt.frame_dirty)
    }

    pub fn mark_frame_dirty() {
        with_rt(|rt| rt.frame_dirty = true);
    }
}
