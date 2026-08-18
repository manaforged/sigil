//! Executor + completion-queue surface on [`Runtime`].
//!
//! Extracted from `runtime.rs` to keep that file under the 400-line
//! cap. The `impl Runtime` block here exposes four methods:
//!
//! - [`Runtime::install_executor`] — wire up the executor +
//!   main-thread dispatcher pair (DR-048 §5).
//! - [`Runtime::enqueue_completion`] — public seam for
//!   [`crate::async_cx::MainThreadDispatcher`] adapters to push
//!   completion callbacks after the cross-thread hop.
//! - [`Runtime::pump_completions`] — test-only drain without an
//!   executor in the loop.
//! - [`Runtime::run_until_idle`] — bounded quiescence drive for
//!   tests that want a deterministic "settle" point.

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
    /// Install an executor + main-thread dispatcher pair (DR-048 §5).
    ///
    /// The reactive crate cannot depend on `sigil-async` (would be
    /// circular); the application wires up the concrete pair at
    /// startup. Calling twice replaces the previous install.
    ///
    /// Must be called from the thread that owns the reactive runtime.
    pub fn install_executor(
        executor: Arc<dyn TaskExecutor>,
        dispatcher: Arc<dyn MainThreadDispatcher>,
    ) {
        with_rt(|rt| {
            rt.executor = Some(executor);
            rt.dispatcher = Some(dispatcher);
        });
    }

    /// Install a `!Send`-future local executor (W4.99.B3).
    ///
    /// Companion to [`Runtime::install_executor`] for the
    /// [`crate::EntityCx::spawn_local`] path. The local executor drives
    /// futures that hold `!Send` state across `.await` (the canonical
    /// case is a `Rc<RefCell<_>>` captured by the future body).
    ///
    /// Production install: a `tokio::task::LocalSet`-backed executor
    /// from `sigil-async`. Tests install a deterministic fixture.
    ///
    /// Calling twice replaces the previous install. Must be called from
    /// the thread that owns the reactive runtime.
    pub fn install_local_executor(executor: Arc<dyn LocalTaskExecutor>) {
        with_rt(|rt| {
            rt.local_executor = Some(executor);
        });
    }

    /// Push `callback` onto the main-thread completion queue under
    /// `owner` (DR-048 §4).
    ///
    /// `callback` runs on the main thread at the top of the next
    /// [`Runtime::begin_effect`] (also drained by
    /// [`Runtime::pump_completions`] / [`Runtime::run_until_idle`]).
    /// If `owner` has been disposed by the time the queue drains, the
    /// callback is dropped silently (DR-007 invariant 4).
    ///
    /// This is the public seam that
    /// `MainThreadDispatcher::post_to_main` adapters call after they
    /// have ferried the closure across the worker→main thread boundary.
    /// The reactive crate cannot push from a worker thread directly
    /// because `crate::runtime::RuntimeState` is thread-local; the
    /// adapter is responsible for the cross-thread hop, then this
    /// entry point takes over on the main thread.
    ///
    /// Must be called from the thread that owns the reactive runtime.
    pub fn enqueue_completion(owner: OwnerId, callback: Box<dyn FnOnce() + Send + 'static>) {
        push_pending_completion(owner.0, callback);
    }

    /// Drain `pending_completions` without driving an executor.
    ///
    /// Used by tests that push completions directly (no Tokio worker
    /// in the loop). Transitions through `Effect` so handler bodies
    /// see the right phase, then back to `Idle`. Also drains the
    /// observer queue so any `cx.notify` issued by handlers fans out
    /// before return.
    ///
    /// # Panics
    ///
    /// Debug-panics if the current phase is not `Idle` on entry.
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

    /// Drive the runtime until quiescent (DR-048 §8).
    ///
    /// Loops `Effect`-phase fan-out + observer drain until both
    /// `pending_completions` and the observer queue are empty AND no
    /// new completion landed during the pass. Bounded by
    /// `MAX_QUIESCENCE_PASSES`.
    ///
    /// Test-only seam — production frame drivers use the explicit
    /// `begin_effect → begin_layout → begin_paint → end_frame` sequence.
    /// Outside that pipeline, completions and observers sit on their
    /// queues until the next frame.
    ///
    /// # Panics
    ///
    /// Panics if the runtime does not reach quiescence within
    /// `MAX_QUIESCENCE_PASSES` iterations — surface for a runaway
    /// completion cycle.
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
