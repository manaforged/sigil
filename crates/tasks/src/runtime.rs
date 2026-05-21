//! [`AsyncRuntime`] — the concrete [`Scheduler`] adapter.
//!
//! Owns one [`tokio::runtime::Runtime`], one MPSC payload channel,
//! and a per-owner `AbortHandle` registry. Owner disposal aborts
//! spawned futures **synchronously** via an `on_cleanup` hook
//! registered at spawn time; the registry remains as belt-and-
//! braces drain-time cleanup for race cases. See the crate-level
//! docs for the full picture.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, Mutex};

use hal::Scheduler;
use reactive::{BoxMainCallback, OwnerId};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::{AbortHandle, JoinHandle};

use crate::handle::OwnerHandle;
use crate::reactive_bridge::{
    ReactiveBridge, build_bridge, drain_into_reactive, replace_bridge_waker,
};

/// Error surface for the scheduler.
///
/// Today the only variant fires when the main-thread dispatcher has
/// been torn down while a background task still holds a handle.
/// The [`Full`][ScheduleError::Full] variant is reserved for the
/// future bounded-queue behaviour documented in
/// [`hal::scheduler`]; producing it from the current unbounded
/// adapter would be a bug, but the variant stays in the public
/// surface so that the bounded adapter (later phase) can start
/// returning it without breaking callers.
#[derive(Debug)]
pub enum ScheduleError {
    /// The main-thread receiver has been dropped. The callback was
    /// not enqueued; the caller's closure has already been dropped.
    Closed,
    /// Reserved for future bounded-queue behaviour.
    Full,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => f.write_str("main-thread dispatcher is closed"),
            Self::Full => f.write_str("dispatcher queue is full"),
        }
    }
}

impl Error for ScheduleError {}

/// One enqueued unit of work: a callback plus the owner it is
/// scoped to.
///
/// Lives only between `enqueue_main` and `drain`. The `Box<dyn
/// FnOnce...>` is dropped without being called if the owner is
/// disposed between those two points.
pub(crate) struct DispatchMessage {
    pub(crate) owner: OwnerHandle,
    pub(crate) callback: Box<dyn FnOnce() + Send + 'static>,
}

/// `Send + Sync + Clone` handle to the main-thread dispatcher.
///
/// Background code holds a `Dispatcher` — not an `&AsyncRuntime`,
/// which is `!Sync`. A `Dispatcher` carries the MPSC sender half and
/// the waker callback, both of which are inexpensive to clone.
///
/// Obtain one via [`AsyncRuntime::dispatcher`]. Background tasks
/// spawned with [`AsyncRuntime::spawn`] (via the
/// [`Scheduler`][hal::Scheduler] trait) move a `Dispatcher` clone
/// into their `async move` body and use it to call
/// [`dispatch_to_owner`][crate::dispatch_to_owner].
///
/// Cloning is cheap — one `Arc` increment for the waker and one
/// `UnboundedSender::clone` for the channel.
#[derive(Clone)]
pub struct Dispatcher {
    pub(crate) tx: UnboundedSender<DispatchMessage>,
    pub(crate) waker: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl fmt::Debug for Dispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dispatcher").finish_non_exhaustive()
    }
}

impl Dispatcher {
    /// Push `message` onto the main-thread queue and fire the waker.
    pub(crate) fn send(&self, message: DispatchMessage) -> Result<(), ScheduleError> {
        self.tx.send(message).map_err(|_| ScheduleError::Closed)?;
        (self.waker)();
        Ok(())
    }
}

// Compile-time guarantee that `Dispatcher` is `Send + Sync + Clone +
// 'static`. Background tasks rely on this — move a `Dispatcher` into
// an `async move` and clone it to fan out. If any field stops
// satisfying these bounds the build breaks here.
const _: fn() = || {
    const fn assert_send_sync_clone_static<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync_clone_static::<Dispatcher>();
};

/// The tokio-runtime-backed [`Scheduler`] implementation.
///
/// One per `App`. Constructed synchronously on the main thread;
/// dropping it tears down the Tokio runtime via a blocking shutdown
/// (Tokio's default `Drop` behaviour). See the crate-level docs for
/// the invariants this type upholds.
///
/// # Layout
///
/// - `rt`: the owned multi-thread Tokio runtime. Stays alive for the
///   whole `AsyncRuntime` lifetime; spawned tasks use its handle.
/// - `sender`: cloned into every enqueue site. Carries the MPSC sender
///   half and the waker.
/// - `rx`: the MPSC receiver, held by the dispatcher. Drained from
///   the main thread at frame start via
///   [`Scheduler::drain`][hal::Scheduler::drain].
/// - `owner_tasks`: per-owner `AbortHandle` registry. Updated from
///   any thread on spawn; swept on drain so dead owners' tasks are
///   aborted promptly.
pub struct AsyncRuntime {
    rt: Runtime,
    sender: Dispatcher,
    rx: UnboundedReceiver<DispatchMessage>,
    owner_tasks: Arc<Mutex<HashMap<OwnerId, Vec<AbortHandle>>>>,
    /// G5 (DR-048 §5) — sender half of the reactive bridge channel.
    /// Background tasks calling `MainThreadDispatcher::post_to_main`
    /// push `(OwnerId, BoxMainCallback)` pairs through this; the main
    /// thread drains via `pump_completions_into_reactive`.
    bridge: ReactiveBridge,
    /// G5 (DR-048 §5) — receiver half of the reactive bridge channel.
    /// Held by `AsyncRuntime` and drained on the main thread.
    bridge_rx: UnboundedReceiver<(OwnerId, BoxMainCallback)>,
}

impl fmt::Debug for AsyncRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncRuntime").finish_non_exhaustive()
    }
}

/// Default no-op waker. Replaced by `sigil-shell` with a closure that
/// calls `EventLoopProxy::wake_up()`.
fn noop_waker() -> Arc<dyn Fn() + Send + Sync + 'static> {
    Arc::new(|| {})
}

impl AsyncRuntime {
    /// Build a new async runtime with a fresh multi-thread Tokio
    /// runtime.
    ///
    /// The waker starts as a no-op. `sigil-shell` replaces it via
    /// [`set_waker`][AsyncRuntime::set_waker] once an `EventLoopProxy`
    /// is available. Tests can inject a counter-waker the same way.
    ///
    /// # Errors
    ///
    /// Returns any error from `tokio::runtime::Builder::build()` —
    /// in practice, OS failure to spawn worker threads.
    pub fn new() -> std::io::Result<Self> {
        let rt = Builder::new_multi_thread().enable_all().build()?;
        let (tx, rx) = unbounded_channel();
        let (bridge, bridge_rx) = build_bridge();
        Ok(Self {
            rt,
            sender: Dispatcher {
                tx,
                waker: noop_waker(),
            },
            rx,
            owner_tasks: Arc::new(Mutex::new(HashMap::new())),
            bridge,
            bridge_rx,
        })
    }

    /// Replace the waker closure.
    ///
    /// Called by `sigil-shell` with a closure that wakes the winit
    /// event loop via `EventLoopProxy::wake_up()`. Tests use it to
    /// install counters that assert the waker fires per enqueue.
    pub fn set_waker<F>(&mut self, waker: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.sender.waker = Arc::new(waker);
    }

    /// G5 (DR-048 §5) — clone the reactive-bridge sender half.
    ///
    /// The returned [`ReactiveBridge`] is `Send + Sync + Clone` and
    /// implements [`reactive::MainThreadDispatcher`]. The
    /// application installs this via
    /// [`reactive::Runtime::install_executor`] (paired with the
    /// [`crate::TokioExecutor`] from [`AsyncRuntime::tokio_executor`])
    /// at startup; background tasks then post completion callbacks
    /// through it.
    #[must_use]
    pub fn reactive_bridge(&self) -> ReactiveBridge {
        self.bridge.clone()
    }

    /// G5 (DR-048 §5) — mint a [`crate::TokioExecutor`] over this
    /// runtime's Tokio handle.
    ///
    /// The returned executor implements [`reactive::TaskExecutor`]
    /// and shares the same Tokio multi-thread runtime as
    /// `AsyncRuntime`'s [`hal::Scheduler`] surface — there is
    /// only one Tokio runtime per `AsyncRuntime`.
    #[must_use]
    pub fn tokio_executor(&self) -> crate::TokioExecutor {
        crate::TokioExecutor::new(self.rt.handle().clone())
    }

    /// G5 (DR-048 §5) — replace the reactive-bridge waker closure.
    ///
    /// Same shape as [`AsyncRuntime::set_waker`] but for the bridge
    /// channel (different MPSC than the [`hal::Scheduler`]
    /// queue). Tests can swap a counter waker to assert the worker
    /// fired its waker per `post_to_main` call.
    pub fn set_bridge_waker<F>(&mut self, waker: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        replace_bridge_waker(&mut self.bridge, waker);
    }

    /// G5 (DR-048 §5) — drain the reactive-bridge MPSC, forwarding
    /// each `(owner, callback)` entry to
    /// [`reactive::Runtime::enqueue_completion`].
    ///
    /// Must be called on the main thread (the reactive runtime is
    /// thread-local). Returns the number of entries moved into the
    /// reactive `pending_completions` queue. Bounded per call — at
    /// most the queue length observed at entry, so a fast producer
    /// can't starve the caller.
    ///
    /// The frame loop invokes this just before
    /// [`reactive::Runtime::begin_effect`], which then drains
    /// the freshly-forwarded completions as the first step of the
    /// Effect phase.
    pub fn pump_completions_into_reactive(&mut self) -> usize {
        drain_into_reactive(&mut self.bridge_rx)
    }

    /// A cloneable, `Send + Sync` dispatcher handle.
    ///
    /// Background tasks hold a `Dispatcher` and use it to call
    /// [`dispatch_to_owner`][crate::dispatch_to_owner]. The main
    /// thread keeps the `AsyncRuntime` and drains at frame start;
    /// the two halves communicate through the dispatcher's MPSC.
    #[must_use]
    pub fn dispatcher(&self) -> Dispatcher {
        self.sender.clone()
    }

    /// A cloneable handle to the per-owner abort registry.
    ///
    /// Exposed to `crate::dispatch` so `spawn_to_signal` can record
    /// its own abort handle without going through the `Scheduler`
    /// trait (which would force a round-trip through `Self::Task`).
    pub(crate) fn owner_tasks(&self) -> Arc<Mutex<HashMap<OwnerId, Vec<AbortHandle>>>> {
        Arc::clone(&self.owner_tasks)
    }

    /// A handle to the underlying Tokio runtime.
    ///
    /// Used internally by `crate::dispatch` to spawn futures whose
    /// cancellation is hooked into the shared `owner_tasks` registry.
    /// No public surface exposes this — user code spawns via
    /// [`spawn_to_signal`][crate::spawn_to_signal] or the
    /// [`Scheduler::spawn`][hal::Scheduler::spawn] impl below.
    pub(crate) fn tokio_handle(&self) -> tokio::runtime::Handle {
        self.rt.handle().clone()
    }

    /// Number of pending (undrained) main-thread messages.
    ///
    /// Test-only. The real app drives this through the event loop's
    /// drain cadence.
    #[must_use]
    #[doc(hidden)]
    pub fn pending_count(&self) -> usize {
        self.rx.len()
    }
}

/// Record `abort` against `owner` in the registry. Called from the
/// runtime's spawn path.
pub(crate) fn register_abort(
    registry: &Mutex<HashMap<OwnerId, Vec<AbortHandle>>>,
    owner: OwnerId,
    abort: AbortHandle,
) {
    let mut map = registry
        .lock()
        .expect("owner_tasks mutex poisoned by a panicking holder");
    map.entry(owner).or_default().push(abort);
}

/// Drop every abort handle whose target task has already finished,
/// and every entry whose owner has been disposed.
///
/// Called at the start of [`Scheduler::drain`] so cancellation of
/// disposed owners happens promptly (next frame after disposal) and
/// the registry doesn't grow unboundedly as short-lived tasks pile
/// up handles.
pub(crate) fn sweep_owner_tasks(registry: &Mutex<HashMap<OwnerId, Vec<AbortHandle>>>) {
    let mut map = registry
        .lock()
        .expect("owner_tasks mutex poisoned by a panicking holder");
    map.retain(|&owner_id, handles| {
        if !reactive::is_live(owner_id) {
            // Owner has been disposed — abort every still-running
            // task and drop the entry.
            for h in handles.drain(..) {
                h.abort();
            }
            return false;
        }
        // Live owner — purge finished handles to keep the vec small.
        handles.retain(|h| !h.is_finished());
        !handles.is_empty()
    });
}

impl Scheduler for AsyncRuntime {
    type Error = ScheduleError;
    type OwnerHandle = OwnerHandle;
    type SpawnTask = JoinHandle<()>;

    fn enqueue_main(
        &self,
        owner: Self::OwnerHandle,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), Self::Error> {
        // Fire-and-forget: success means the callback was successfully
        // pushed onto the main-thread queue. Actual execution happens
        // on the next `drain`. Callers that need to observe the
        // callback ran should use their own synchronisation inside the
        // callback body — see the trait docs for invariant (4).
        self.sender.send(DispatchMessage { owner, callback: f })
    }

    /// Spawn a future on the Tokio runtime, scoped to `owner`.
    ///
    /// Must be called on the thread that owns the reactive runtime.
    /// At spawn time this registers an `on_cleanup` hook against
    /// `owner` so that disposing the owner aborts the task
    /// synchronously, without waiting for the next `drain`. The
    /// existing `owner_tasks` registry + `drain`-time sweep remain
    /// as belt-and-braces fallback for race cases.
    ///
    /// If `owner` is already disposed, the future is dropped unpolled
    /// and the returned `JoinHandle` reports as finished so callers
    /// can detect the stale-owner case via `.is_finished()`.
    fn spawn<F>(&self, owner: Self::OwnerHandle, future: F) -> Self::SpawnTask
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // Stale-owner fast path: drop the future unpolled rather than
        // let it run unanchored. The returned JoinHandle is for an
        // already-completed empty future, so callers that cared about
        // the spawn can observe `is_finished() == true` immediately.
        if !owner.is_alive() {
            drop(future);
            return self.rt.spawn(async {});
        }

        let join = self.rt.spawn(future);
        let abort = join.abort_handle();

        // Register the abort handle against the owner for belt-and-
        // braces drain-time cleanup (covers the narrow race where
        // `on_cleanup` registration might be missed).
        register_abort(&self.owner_tasks, owner.id(), abort.clone());

        // Register a synchronous cleanup so that disposing the owner
        // aborts the task the instant disposal happens — not at the
        // next drain. This is the main fix for the "disposal doesn't
        // cancel until drain" bug. `with_owner_by_id` scopes the
        // `on_cleanup` registration to `owner` without requiring us
        // to be inside that owner's `with` closure.
        let registered = reactive::owner_by_id(owner.id(), move || {
            reactive::on_cleanup(move || {
                abort.abort();
            });
        });
        // `registered == false` would mean the owner died between the
        // `is_alive` check above and this call — in that case the
        // abort handle in `owner_tasks` will still fire on the next
        // drain's sweep. No action needed here.
        let _ = registered;

        join
    }

    fn drain(&mut self) -> usize {
        // Sweep first so disposed-owner tasks are aborted before we
        // process any of their possibly-still-queued callbacks.
        sweep_owner_tasks(&self.owner_tasks);

        // Bounded per-frame budget. We snapshot the queue length at
        // drain-start and process exactly that many callbacks. This
        // guarantees that a self-rescheduling callback or a fast
        // background producer cannot starve the frame: anything
        // enqueued *during* this drain defers to the next cycle.
        //
        // Invariant (2): `rx.len()` at drain-start is an upper bound
        // on callbacks processed this cycle, NOT a complete count of
        // truly-pending work — more work may be enqueued during or
        // after drain, visible to the next drain call.
        let budget = self.rx.len();
        let mut ran = 0_usize;
        for _ in 0..budget {
            let Ok(msg) = self.rx.try_recv() else {
                // Fewer messages than the snapshot — shouldn't happen
                // for a single-reader drain, but breaking early is
                // correct regardless.
                break;
            };
            if msg.owner.is_alive() {
                // Run the callback with the dispatched owner set as
                // the current reactive owner so any `on_cleanup` or
                // reactive-write inside the closure parents to the
                // right scope. `with_owner_by_id` returns false if
                // the owner died between the liveness check above
                // and the scope switch; in that case the callback
                // is dropped silently.
                let callback = msg.callback;
                // `Box<dyn FnOnce() + Send + 'static>` satisfies
                // `impl FnOnce()` directly (the `Box<dyn FnOnce>`
                // FnOnce impl) — no extra closure wrapping needed.
                let ran_inner = reactive::owner_by_id(msg.owner.id(), callback);
                if ran_inner {
                    ran += 1;
                }
            }
            // Dead owner: drop the callback without running.
        }
        ran
    }
}
