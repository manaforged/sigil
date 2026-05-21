//! W3.3-impl — integration tests for G5 `Task<T>` + `AsyncContext` +
//! the executor-install seam (DR-048).
//!
//! Covers `Context::spawn` end-to-end: a TestExecutor backed by a
//! multi-thread tokio runtime supplies real `AbortHandle`s; a
//! TestDispatcher buffers main-thread callbacks in a `Mutex<VecDeque>`
//! that the test code drains explicitly. The drain seam keeps the
//! tests deterministic — no `tokio::time` feature, no race window
//! polling on wall-clock sleeps.
//!
//! See DR-048 §8 (test seam) and §4 (completion routing).

#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::ignored_unit_patterns,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::redundant_closure_for_method_calls,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::redundant_clone
)]

use actions as _;
use criterion as _;
use smallvec as _;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reactive::{
    AsyncContext, BoxMainCallback, BoxSendFuture, DispatchError, DrawPhase, Entity,
    MainThreadDispatcher, Owner, OwnerId, Runtime, Task, TaskAwaitError, TaskExecutor,
};
use tokio::runtime::Builder as TokioBuilder;
use tokio::sync::oneshot;
use tokio::task::AbortHandle;

// ---- Test fixture: TestExecutor + TestDispatcher --------------------

/// Single tokio multi-thread runtime kept inside an Arc so the executor
/// can hand out abort handles. Each test builds its own to avoid
/// cross-test state.
struct TestExecutor {
    rt: Option<tokio::runtime::Runtime>,
}

impl TestExecutor {
    fn new() -> Arc<Self> {
        let rt = TokioBuilder::new_multi_thread()
            .worker_threads(2)
            .build()
            .expect("build test tokio runtime");
        Arc::new(Self { rt: Some(rt) })
    }

    fn handle(&self) -> &tokio::runtime::Runtime {
        self.rt.as_ref().expect("rt present until Drop")
    }
}

impl Drop for TestExecutor {
    fn drop(&mut self) {
        // Force background shutdown so the test thread doesn't block
        // on worker join. Idempotent — Option::take returns None on
        // double-drop.
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}

impl TaskExecutor for TestExecutor {
    fn spawn(&self, _owner: OwnerId, fut: BoxSendFuture) -> AbortHandle {
        let handle = self.handle().spawn(fut);
        handle.abort_handle()
    }
}

/// Buffers callbacks posted by the worker until the test drains them.
#[derive(Default)]
struct TestDispatcher {
    queue: Mutex<VecDeque<(OwnerId, BoxMainCallback)>>,
}

impl TestDispatcher {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn pending(&self) -> usize {
        self.queue.lock().expect("dispatcher mutex").len()
    }

    /// Drain every buffered callback by running it inline on the
    /// current thread. The test thread IS the reactive runtime's
    /// main thread, so this satisfies the "post_to_main" contract.
    fn drain_inline(&self) -> usize {
        let drained: Vec<(OwnerId, BoxMainCallback)> = {
            let mut guard = self.queue.lock().expect("dispatcher mutex");
            guard.drain(..).collect()
        };
        let count = drained.len();
        for (_owner, cb) in drained {
            cb();
        }
        count
    }

    /// Busy-wait until at least `n` callbacks are buffered or the
    /// deadline elapses. Returns whether the threshold was reached.
    fn wait_for_pending(&self, n: usize, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.pending() >= n {
                return true;
            }
            thread::sleep(Duration::from_millis(1));
        }
        self.pending() >= n
    }
}

impl MainThreadDispatcher for TestDispatcher {
    fn post(&self, owner: OwnerId, callback: BoxMainCallback) -> Result<(), DispatchError> {
        self.queue
            .lock()
            .expect("dispatcher mutex")
            .push_back((owner, callback));
        Ok(())
    }
}

/// Spin up an executor + dispatcher pair, install them, and run `f`
/// inside a `Runtime::scope`. Each test gets a fresh pair.
fn with_async_fixture<F>(name: &'static str, f: F)
where
    F: FnOnce(&Arc<TestExecutor>, &Arc<TestDispatcher>),
{
    Runtime::scope(name, || {
        let executor = TestExecutor::new();
        let dispatcher = TestDispatcher::new();
        let executor_dyn: Arc<dyn TaskExecutor> = executor.clone();
        let dispatcher_dyn: Arc<dyn MainThreadDispatcher> = dispatcher.clone();
        Runtime::install_executor(executor_dyn, dispatcher_dyn);
        f(&executor, &dispatcher);
    });
}

/// Test entity payload — wraps an atomic counter shared with the
/// background future.
struct Counter {
    n: Arc<AtomicUsize>,
}

impl Counter {
    fn new() -> (Self, Arc<AtomicUsize>) {
        let n = Arc::new(AtomicUsize::new(0));
        (Self { n: n.clone() }, n)
    }
}

/// Busy-wait for the worker to set `flag` to a non-zero value, or
/// time out. Used by tests that need to observe a future's body
/// running without a tokio::time::sleep dependency.
fn wait_for_atomic(flag: &AtomicUsize, target: usize, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if flag.load(Ordering::SeqCst) >= target {
            return true;
        }
        thread::sleep(Duration::from_millis(1));
    }
    flag.load(Ordering::SeqCst) >= target
}

// ---- Tests -----------------------------------------------------------

// 1. — spawn drives the future, dispatcher receives the update closure.
#[test]
fn spawn_runs_future_to_completion() {
    with_async_fixture("spawn_runs_to_completion", |_exec, dispatcher| {
        let (counter, n) = Counter::new();
        let entity = Entity::<Counter>::new("counter", |_| counter);
        let task: Task<()> = entity.update(|_, cx| {
            cx.spawn(|acx: AsyncContext<Counter>| async move {
                acx.update(|this, _cx| {
                    this.n.fetch_add(1, Ordering::SeqCst);
                })
                .await
                .expect("entity alive");
            })
        });
        assert!(
            dispatcher.wait_for_pending(1, Duration::from_secs(2)),
            "dispatcher received update callback within timeout"
        );
        let moved = dispatcher.drain_inline();
        assert_eq!(moved, 1, "exactly one callback was drained");
        assert_eq!(
            n.load(Ordering::SeqCst),
            1,
            "future body mutated the counter through cx.update"
        );
        task.detach();
    });
}

// 2. — Completion drains on Idle by default; phase observed is Idle.
#[test]
fn spawn_completion_phase_under_test_drain() {
    with_async_fixture("completion_phase", |_exec, dispatcher| {
        let phase_seen: Arc<Mutex<Option<DrawPhase>>> = Arc::new(Mutex::new(None));
        let phase_clone = phase_seen.clone();
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("phase_probe", |_| counter);
        let task: Task<()> = entity.update(|_, cx| {
            cx.spawn(move |acx: AsyncContext<Counter>| async move {
                acx.update(move |_this, _cx| {
                    *phase_clone.lock().expect("phase_seen") = Some(Runtime::current_phase());
                })
                .await
                .expect("alive");
            })
        });
        assert!(dispatcher.wait_for_pending(1, Duration::from_secs(2)));
        dispatcher.drain_inline();
        let observed = *phase_seen.lock().expect("phase_seen");
        assert_eq!(
            observed,
            Some(DrawPhase::Idle),
            "drain_inline runs the callback under the test's Idle phase"
        );
        task.detach();
    });
}

// 3. — Dropping the Task aborts the future before it can run.
#[test]
fn task_drop_cancels_future() {
    with_async_fixture("drop_cancels", |_exec, dispatcher| {
        let flag = Arc::new(AtomicUsize::new(0));
        let flag_clone = flag.clone();
        // Signalled-future shape: the future awaits a oneshot we
        // never send. Dropping the Task aborts the future at its
        // await-point; the post-await flag mutation never runs.
        let (_tx, rx) = oneshot::channel::<()>();
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("droppable", |_| counter);
        {
            let task: Task<()> = entity.update(|_, cx| {
                cx.spawn(move |_acx: AsyncContext<Counter>| async move {
                    let _ = rx.await;
                    flag_clone.fetch_add(1, Ordering::SeqCst);
                })
            });
            drop(task); // explicit drop → abort
        }
        // Give the worker time to observe the abort. We do NOT send
        // through tx — the future stays parked on rx.await and the
        // abort drops the future at that yield point.
        thread::sleep(Duration::from_millis(80));
        dispatcher.drain_inline();
        assert_eq!(
            flag.load(Ordering::SeqCst),
            0,
            "future body did not run past its await after Task drop-cancel"
        );
    });
}

// 4. — `.detach()` keeps the task alive past handle drop.
#[test]
fn detach_keeps_task_alive() {
    with_async_fixture("detach_alive", |_exec, dispatcher| {
        let flag = Arc::new(AtomicUsize::new(0));
        let flag_clone = flag.clone();
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("detached", |_| counter);
        let task: Task<()> = entity.update(|_, cx| {
            cx.spawn(move |_acx: AsyncContext<Counter>| async move {
                flag_clone.fetch_add(1, Ordering::SeqCst);
            })
        });
        task.detach(); // consume without abort
        assert!(
            wait_for_atomic(&flag, 1, Duration::from_secs(2)),
            "detached future ran to completion"
        );
        dispatcher.drain_inline();
    });
}

// 5. — Owner-cascade abort: entity's owner disposal cancels the task.
#[test]
fn owner_cascade_cancels_owned_tasks() {
    Runtime::scope("owner_cascade", || {
        let executor = TestExecutor::new();
        let dispatcher = TestDispatcher::new();
        let executor_dyn: Arc<dyn TaskExecutor> = executor.clone();
        let dispatcher_dyn: Arc<dyn MainThreadDispatcher> = dispatcher.clone();
        Runtime::install_executor(executor_dyn, dispatcher_dyn);
        let flag = Arc::new(AtomicUsize::new(0));
        let flag_clone = flag.clone();
        let (_tx, rx) = oneshot::channel::<()>();

        let child = Owner::new(
            "child_scope",
            Some(Owner::current().expect("scope establishes owner")),
        );
        let task: Task<()> = child.with(|| {
            let (counter, _) = Counter::new();
            let entity = Entity::<Counter>::new("inside_child", |_| counter);
            entity.update(|_, cx| {
                cx.spawn(move |_acx: AsyncContext<Counter>| async move {
                    let _ = rx.await;
                    flag_clone.fetch_add(1, Ordering::SeqCst);
                })
            })
        });
        // Owner disposal runs cleanups → abort fires.
        child.dispose();
        task.detach(); // remove RAII to isolate owner-cascade behaviour

        thread::sleep(Duration::from_millis(80));
        dispatcher.drain_inline();
        assert_eq!(
            flag.load(Ordering::SeqCst),
            0,
            "future body never ran after owner-cascade abort"
        );
    });
}

// 6. — `update` returns Err(EntityGone) when the dispatcher's MPSC is
//      closed (DR-007 invariant 5 — runtime shutting down).
//
// NOTE: the "entity literally removed from arena → EntityGone" path
// exists in `run_update_on_main` but cannot be exercised end-to-end
// today because `dispose_owner` does not yet evict the owner's
// entities from `rt.entities` (DR-045 followup, separate from W3.3).
// Until that wiring lands, the dispatcher-closed surface is the
// only externally-observable EntityGone path.
#[test]
fn update_returns_err_on_dispatcher_closed() {
    /// Dispatcher whose `post_to_main` always returns `Closed` — models
    /// the "runtime tearing down" case.
    struct ClosedDispatcher;
    impl MainThreadDispatcher for ClosedDispatcher {
        fn post(&self, _owner: OwnerId, _callback: BoxMainCallback) -> Result<(), DispatchError> {
            Err(DispatchError::Closed)
        }
    }

    Runtime::scope("dispatcher_closed", || {
        let executor = TestExecutor::new();
        let exec_dyn: Arc<dyn TaskExecutor> = executor.clone();
        let disp_dyn: Arc<dyn MainThreadDispatcher> = Arc::new(ClosedDispatcher);
        Runtime::install_executor(exec_dyn, disp_dyn);

        let captured: Arc<Mutex<Option<AsyncContext<Counter>>>> = Arc::new(Mutex::new(None));
        let captured_for_spawn = captured.clone();
        let (block_tx, block_rx) = oneshot::channel::<()>();
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("closed_dispatch", |_| counter);
        let task: Task<()> = entity.update(|_, cx| {
            cx.spawn(move |acx: AsyncContext<Counter>| {
                *captured_for_spawn.lock().expect("captured") = Some(acx);
                async move {
                    let _ = block_rx.await;
                }
            })
        });
        // Wait for the worker to capture acx.
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && captured.lock().expect("captured").is_none() {
            thread::sleep(Duration::from_millis(2));
        }
        let acx = captured
            .lock()
            .expect("captured")
            .clone()
            .expect("acx captured before timeout");
        // Probe: acx.update must return Err because the dispatcher is
        // closed. Run on the worker so the await actually exercises
        // the dispatch path.
        let result_is_err = executor.handle().block_on(async move {
            let res = acx.update(|_this: &mut Counter, _cx| ()).await;
            res.is_err()
        });
        assert!(
            result_is_err,
            "cx.update with a closed dispatcher returned Err(EntityGone)"
        );
        let _ = block_tx.send(());
        task.detach();
    });
}

// 7. — pump_completions is a no-op when nothing is queued.
#[test]
fn pump_completions_no_op_when_empty() {
    Runtime::scope("pump_empty", || {
        let executor = TestExecutor::new();
        let dispatcher = TestDispatcher::new();
        let exec_dyn: Arc<dyn TaskExecutor> = executor.clone();
        let dispatcher_dyn: Arc<dyn MainThreadDispatcher> = dispatcher.clone();
        Runtime::install_executor(exec_dyn, dispatcher_dyn);
        Runtime::pump_completions();
        assert_eq!(
            Runtime::current_phase(),
            DrawPhase::Idle,
            "pump_completions returns runtime to Idle"
        );
    });
}

// 8. — run_until_idle is a no-op when nothing is queued.
#[test]
fn run_until_idle_quiescent_baseline() {
    Runtime::scope("run_until_idle_baseline", || {
        let executor = TestExecutor::new();
        let dispatcher = TestDispatcher::new();
        let executor_dyn: Arc<dyn TaskExecutor> = executor.clone();
        let dispatcher_dyn: Arc<dyn MainThreadDispatcher> = dispatcher.clone();
        Runtime::install_executor(executor_dyn, dispatcher_dyn);
        Runtime::run_until_idle();
        assert_eq!(
            Runtime::current_phase(),
            DrawPhase::Idle,
            "run_until_idle leaves runtime in Idle"
        );
    });
}

// 9. — Multiple completions all arrive (set equality, not order — the
//      multi-thread tokio runtime gives no per-spawn order guarantee).
#[test]
fn multiple_completions_all_arrive() {
    with_async_fixture("ordered_completions", |_exec, dispatcher| {
        let log: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
        let log_a = log.clone();
        let log_b = log.clone();
        let log_c = log.clone();
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("order_root", |_| counter);
        let t1: Task<()> = entity.update(|_, cx| {
            cx.spawn(move |acx| async move {
                acx.update(move |_this: &mut Counter, _cx| {
                    log_a.lock().expect("log").push(1);
                })
                .await
                .expect("alive");
            })
        });
        let t2: Task<()> = entity.update(|_, cx| {
            cx.spawn(move |acx| async move {
                acx.update(move |_this: &mut Counter, _cx| {
                    log_b.lock().expect("log").push(2);
                })
                .await
                .expect("alive");
            })
        });
        let t3: Task<()> = entity.update(|_, cx| {
            cx.spawn(move |acx| async move {
                acx.update(move |_this: &mut Counter, _cx| {
                    log_c.lock().expect("log").push(3);
                })
                .await
                .expect("alive");
            })
        });
        assert!(dispatcher.wait_for_pending(3, Duration::from_secs(2)));
        dispatcher.drain_inline();
        let mut sorted = log.lock().expect("log").clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            vec![1, 2, 3],
            "every spawn's completion landed exactly once"
        );
        t1.detach();
        t2.detach();
        t3.detach();
    });
}

// 10. — Spawn during Effect phase is legal — the canonical path for
//      action handlers, observer callbacks, and frame-driver hooks.
#[test]
fn effect_phase_spawn_is_legal() {
    with_async_fixture("effect_spawn", |_exec, dispatcher| {
        let (counter, n) = Counter::new();
        let entity = Entity::<Counter>::new("effect_root", |_| counter);
        let n_for_future = n.clone();
        Runtime::begin_effect();
        let task: Task<()> = entity.update(|_, cx| {
            cx.spawn(move |_acx: AsyncContext<Counter>| async move {
                n_for_future.fetch_add(1, Ordering::SeqCst);
            })
        });
        // Walk through the rest of the frame so the phase returns to
        // Idle cleanly.
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();
        assert!(
            wait_for_atomic(&n, 1, Duration::from_secs(2)),
            "Effect-phase spawn completed without panic"
        );
        dispatcher.drain_inline();
        task.detach();
    });
}

// 11. — Cancel + await_or_cancel returns Err(Cancelled).
#[test]
fn await_or_cancel_after_cancel_returns_cancelled() {
    with_async_fixture("await_or_cancel", |exec, _dispatcher| {
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("cancellable", |_| counter);
        let (_tx, rx) = oneshot::channel::<()>();
        let mut task: Task<()> = entity.update(|_, cx| {
            cx.spawn(move |_acx: AsyncContext<Counter>| async move {
                let _ = rx.await;
            })
        });
        task.cancel();
        let result = exec
            .handle()
            .block_on(async { task.await_or_cancel().await });
        assert!(
            matches!(result, Err(TaskAwaitError::Cancelled)),
            "cancelled task surfaces Cancelled (got {result:?})"
        );
    });
}

// 12. — DR-048 amendment B3: `Context::spawn` is a write seam — every
//       Paint-phase write path panics in debug. The composite message
//       is "Entity::update called during DrawPhase::Paint" because
//       Entity::update fires its own assert_paint_safe first; the
//       spawn-specific guard is verified by source inspection.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "DrawPhase::Paint")]
fn spawn_in_paint_panics() {
    with_async_fixture("spawn_in_paint", |_exec, _dispatcher| {
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("paint_probe", |_| counter);
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        let _task: Task<()> =
            entity.update(|_, cx| cx.spawn(|_acx: AsyncContext<Counter>| async move {}));
        Runtime::end_frame();
    });
}

type ProbeOut = Arc<Mutex<Option<Result<(), reactive::EntityGone>>>>;

/// Spawn a future that captures `acx`, then wait for the capture.
/// Returns the `AsyncContext` and the originating `Task` handle.
fn capture_acx(child: Owner) -> (AsyncContext<Counter>, Task<()>) {
    let captured: Arc<Mutex<Option<AsyncContext<Counter>>>> = Arc::new(Mutex::new(None));
    let captured_for_spawn = captured.clone();
    let task: Task<()> = child.with(|| {
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("evictable", |_| counter);
        entity.update(|_, cx| {
            cx.spawn(move |acx: AsyncContext<Counter>| {
                *captured_for_spawn.lock().expect("captured") = Some(acx);
                async move {}
            })
        })
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && captured.lock().expect("captured").is_none() {
        thread::sleep(Duration::from_millis(2));
    }
    let acx = captured
        .lock()
        .expect("captured")
        .clone()
        .expect("acx captured before timeout");
    (acx, task)
}

/// Drain the dispatcher in a loop on the test thread until the worker
/// stashes a result into `probe_out`. Returns the result or panics on
/// timeout.
fn drain_until_probe_done(
    dispatcher: &TestDispatcher,
    probe_out: &ProbeOut,
) -> Result<(), reactive::EntityGone> {
    let drain_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < drain_deadline {
        dispatcher.drain_inline();
        let taken = probe_out.lock().expect("probe_out").take();
        if let Some(r) = taken {
            return r;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("probe future did not resolve within timeout");
}

// 13. — DR-045 amendment §A4: after the entity's owner disposes, the
//      arena slot is evicted (DR-045 §A1) so `acx.update(...)` returns
//      `Err(EntityGone)`. Prior to the eviction phase this test path
//      did not exist — the slot survived dispose and `update` returned
//      `Ok(())`.
#[test]
fn update_after_entity_gone_returns_err() {
    with_async_fixture("entity_gone", |exec, dispatcher| {
        let child = Owner::new(
            "child_for_eviction",
            Some(Owner::current().expect("scope establishes owner")),
        );
        let (acx, task) = capture_acx(child);
        task.detach();

        // Dispose the child owner. DR-045 §A1 eviction removes the
        // entity slot from rt.entities; the slot's generation has
        // bumped so the EntityKey stored inside `acx` is now stale.
        child.dispose();

        // Issue acx.update from a tokio worker; stash the outcome
        // in a Mutex the test thread polls while draining the
        // dispatcher.
        let probe_out: ProbeOut = Arc::new(Mutex::new(None));
        let probe_out_for_worker = Arc::clone(&probe_out);
        exec.handle().spawn(async move {
            let res = acx.update(|_this: &mut Counter, _cx| ()).await;
            *probe_out_for_worker.lock().expect("probe_out") = Some(res);
        });

        let result = drain_until_probe_done(dispatcher, &probe_out);
        assert!(
            result.is_err(),
            "acx.update after owner dispose must return Err(EntityGone) — slot was evicted (got {result:?})"
        );
    });
}

// 14. — Awaiting a successfully-completed Task yields its result.
#[test]
fn task_await_yields_result() {
    with_async_fixture("await_result", |exec, _dispatcher| {
        let (counter, _) = Counter::new();
        let entity = Entity::<Counter>::new("returns_value", |_| counter);
        let task: Task<u32> = entity
            .update(|_, cx| cx.spawn(move |_acx: AsyncContext<Counter>| async move { 42_u32 }));
        let value = exec.handle().block_on(task);
        assert_eq!(value, 42, "future's return value reached the awaiter");
    });
}
