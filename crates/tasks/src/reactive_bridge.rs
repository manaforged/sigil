//! G5 reactive executor install — concrete `TaskExecutor` +
//! `MainThreadDispatcher` impls bridging Tokio to the reactive
//! crate's `Context::spawn` seam (DR-048 §5).
//!
//! Two types live here:
//!
//! - [`TokioExecutor`] — `Send + Sync + 'static` wrapper around a
//!   [`tokio::runtime::Handle`]. Implements
//!   [`sigil_reactive::TaskExecutor`] by spawning the boxed future on
//!   the Tokio runtime and returning the corresponding
//!   [`AbortHandle`]. Owner-cascade cancellation is wired by the
//!   reactive crate's `Context::spawn` (via an `on_cleanup` registered
//!   against the spawning entity's owner), so this impl does not
//!   duplicate that tracking.
//!
//! - [`ReactiveBridge`] — `Send + Sync + 'static` main-thread
//!   dispatcher. Implements [`sigil_reactive::MainThreadDispatcher`]
//!   by sending `(OwnerId, callback)` pairs across an MPSC channel.
//!   The main thread pulls from the receiver via
//!   [`AsyncRuntime::pump_completions_into_reactive`] and forwards each
//!   entry to [`sigil_reactive::Runtime::enqueue_completion`].
//!
//! The two are kept separate from [`AsyncRuntime`]'s existing
//! [`sigil_hal::Scheduler`] surface (which has different drain
//! semantics — Scheduler drain runs callbacks inline; the bridge
//! routes through `pending_completions` so DR-048 §4 phase rules
//! apply).

use std::sync::Arc;

use sigil_reactive::{
    BoxMainCallback, BoxSendFuture, DispatchError, MainThreadDispatcher, OwnerId, Runtime,
    TaskExecutor,
};
use tokio::runtime::Handle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::AbortHandle;

/// `Send + Sync` executor that spawns boxed futures on a Tokio
/// runtime handle.
///
/// Constructed via [`crate::AsyncRuntime::tokio_executor`].
/// Owner-cascade abort wiring lives in `sigil_reactive::EntityCx::spawn`
/// — the executor only needs to spawn + return the abort handle.
///
/// # Layout
///
/// - `handle`: cloned per spawn; Tokio's `Handle` is cheap to clone
///   (one `Arc::clone`).
pub struct TokioExecutor {
    handle: Handle,
}

impl core::fmt::Debug for TokioExecutor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TokioExecutor").finish_non_exhaustive()
    }
}

impl TokioExecutor {
    /// Wrap a [`tokio::runtime::Handle`] as a `TaskExecutor`.
    #[must_use]
    pub const fn new(handle: Handle) -> Self {
        Self { handle }
    }
}

impl TaskExecutor for TokioExecutor {
    fn spawn(&self, _owner: OwnerId, fut: BoxSendFuture) -> AbortHandle {
        // Owner-cascade abort is registered by `Context::spawn` on the
        // reactive side (an `on_cleanup` against the spawning entity's
        // owner). This executor just spawns + surfaces the abort handle.
        self.handle.spawn(fut).abort_handle()
    }
}

// Compile-time `Send + Sync + 'static` guarantee — `TaskExecutor`
// requires it and we hand out `Arc<dyn TaskExecutor>` to the reactive
// runtime.
const _: fn() = || {
    const fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<TokioExecutor>();
};

/// Main-thread bridge between worker-thread `post_to_main` calls and
/// the reactive crate's `pending_completions` queue.
///
/// `Send + Sync + Clone` so background tasks can hold a clone and post
/// callbacks across thread boundaries. The receiver half lives on the
/// main thread inside [`crate::AsyncRuntime`]; calls to
/// [`crate::AsyncRuntime::pump_completions_into_reactive`] drain the
/// MPSC and forward each entry to
/// [`sigil_reactive::Runtime::enqueue_completion`], which puts the
/// callback on the reactive runtime's `pending_completions` for
/// drain at the top of the next [`Runtime::begin_effect`].
///
/// Cloning is cheap: one `UnboundedSender::clone` + one
/// `Arc::clone` on the waker.
#[derive(Clone)]
pub struct ReactiveBridge {
    tx: UnboundedSender<(OwnerId, BoxMainCallback)>,
    waker: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl core::fmt::Debug for ReactiveBridge {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ReactiveBridge").finish_non_exhaustive()
    }
}

impl MainThreadDispatcher for ReactiveBridge {
    fn post(&self, owner: OwnerId, callback: BoxMainCallback) -> Result<(), DispatchError> {
        self.tx
            .send((owner, callback))
            .map_err(|_send_err| DispatchError::Closed)?;
        (self.waker)();
        Ok(())
    }
}

const _: fn() = || {
    const fn assert_send_sync_clone_static<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync_clone_static::<ReactiveBridge>();
};

/// Build a (sender, receiver, default-no-op-waker) triple.
///
/// Used by [`crate::AsyncRuntime::reactive_bridge`] to mint matching
/// halves: the sender half wraps into a [`ReactiveBridge`] handed to
/// the reactive runtime; the receiver half is retained by
/// [`crate::AsyncRuntime`] for the main-thread drain.
pub(crate) fn build_bridge() -> (
    ReactiveBridge,
    UnboundedReceiver<(OwnerId, BoxMainCallback)>,
) {
    let (tx, rx) = unbounded_channel();
    let waker: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(|| {});
    (ReactiveBridge { tx, waker }, rx)
}

/// Pull every queued `(owner, callback)` pair off `rx` and forward to
/// [`sigil_reactive::Runtime::enqueue_completion`].
///
/// Must be called on the main thread. Returns the number of entries
/// forwarded (informational — tests assert against it).
///
/// Drain is bounded: snapshots the receiver's current length and
/// processes exactly that many, so a fast producer can't starve the
/// caller (mirrors `Scheduler::drain` invariant 2).
pub(crate) fn drain_into_reactive(rx: &mut UnboundedReceiver<(OwnerId, BoxMainCallback)>) -> usize {
    let budget = rx.len();
    let mut moved = 0_usize;
    for _ in 0..budget {
        let Ok((owner, callback)) = rx.try_recv() else {
            break;
        };
        Runtime::enqueue_completion(owner, callback);
        moved += 1;
    }
    moved
}

/// Replace the waker closure on a `ReactiveBridge`.
///
/// Called by `sigil-shell` (via [`crate::AsyncRuntime::set_bridge_waker`])
/// with a closure that wakes the winit event loop via
/// `EventLoopProxy::wake_up()`. Tests may install counter wakers.
pub(crate) fn replace_bridge_waker<F>(bridge: &mut ReactiveBridge, waker: F)
where
    F: Fn() + Send + Sync + 'static,
{
    bridge.waker = Arc::new(waker);
}
