use std::sync::Arc;

use crate::async_cx::{MainThreadDispatcher, TaskExecutor};
use crate::local_executor::LocalTaskExecutor;
use crate::owner::OwnerId;
use crate::phase::DrawPhase;
use crate::runtime::{MAX_QUIESCENCE_PASSES, Runtime, with_rt};
use crate::runtime_drain::{
    drain_observers_to_fixpoint, drain_pending_completions, push_pending_completion,
};

impl Runtime {
    pub fn install_executor(
        executor: Arc<dyn TaskExecutor>,
        dispatcher: Arc<dyn MainThreadDispatcher>,
    ) {
        with_rt(|rt| {
            rt.executor = Some(executor);
            rt.dispatcher = Some(dispatcher);
        });
    }

    pub fn install_local_executor(executor: Arc<dyn LocalTaskExecutor>) {
        with_rt(|rt| {
            rt.local_executor = Some(executor);
        });
    }

    pub fn enqueue_completion(owner: OwnerId, callback: Box<dyn FnOnce() + Send + 'static>) {
        push_pending_completion(owner.0, callback);
    }

    #[doc(hidden)]
    pub fn pump_completions() {
        with_rt(|rt| {
            debug_assert_eq!(
                rt.phase,
                DrawPhase::Idle,
                "Runtime::pump_completions called from phase {:?}, expected Idle",
                rt.phase,
            );
            rt.phase = DrawPhase::Effect;
        });
        drain_pending_completions();
        drain_observers_to_fixpoint();
        with_rt(|rt| rt.phase = DrawPhase::Idle);
    }

    #[doc(hidden)]
    #[allow(
        clippy::panic,
        reason = "DR-048 §8 mandates a panic on run_until_idle divergence — the cap is a safety net for a runaway completion cycle"
    )]
    pub fn run_until_idle() {
        for _pass in 0..MAX_QUIESCENCE_PASSES {
            let had_work = with_rt(|rt| {
                !rt.pending_completions.is_empty() || !rt.pending_observers.is_empty()
            });
            if !had_work {
                return;
            }
            with_rt(|rt| {
                debug_assert_eq!(
                    rt.phase,
                    DrawPhase::Idle,
                    "Runtime::run_until_idle called from phase {:?}, expected Idle",
                    rt.phase,
                );
                rt.phase = DrawPhase::Effect;
            });
            drain_pending_completions();
            drain_observers_to_fixpoint();
            with_rt(|rt| rt.phase = DrawPhase::Idle);
        }
        panic!(
            "Runtime::run_until_idle exceeded MAX_QUIESCENCE_PASSES ({MAX_QUIESCENCE_PASSES}) — \
             suspect a runaway completion or observer cycle"
        );
    }
}
