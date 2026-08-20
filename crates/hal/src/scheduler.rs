//! Scheduler trait — see DR-007.
//!
//! The `Scheduler` trait abstracts the async runtime and the
//! main-thread dispatcher used to publish signal updates and
//! cross-thread callbacks from background work. In production this is
//! implemented over a single `tokio::Runtime` owned by the `App`,
//! plus an MPSC channel that the main-thread loop drains at the start
//! of each frame. Tests implement a deterministic dispatcher that
//! runs work inline.
//!
//! # Contract (finalised in Phase 1.1c `sigil-async`)
//!
//! 1. **Enqueue order is preserved.** Callbacks run in the order they
//!    were enqueued on the main thread. No reordering for coalescing.
//! 2. **Drain happens at the start of each frame**, before layout,
//!    paint, or event dispatch. This guarantees that by the time a
//!    component reads a signal during the frame, all background
//!    writes that completed before frame start are visible. Drain is
//!    **bounded**: it processes at most the callbacks present in the
//!    queue at drain-start. Callbacks enqueued *during* drain (by a
//!    self-rescheduling callback, or a fast background producer) are
//!    deferred to the next drain cycle so a single frame cannot be
//!    starved by an unbounded producer.
//! 3. **Wake coalesces, payload does not.** Multiple `enqueue_main`
//!    calls before the next event-loop iteration collapse into one
//!    delivered `Shell::wake`, but every enqueued callback runs. The
//!    wake channel is a signal, not a buffer.
//! 4. **Cancellation is identity-based and synchronous.** Each
//!    enqueued callback is tied to an `OwnerHandle` (from
//!    `sigil-reactive`). At drain time, if the owner has been disposed
//!    since enqueue, the callback is dropped — never executed, never
//!    logged as an error. Spawned futures are aborted *synchronously*
//!    at owner disposal (via an `on_cleanup` hook registered at spawn
//!    time) rather than waiting for the next drain.
//! 5. **Bounded queue (future).** The payload channel is unbounded in
//!    the current adapter; bounded behaviour with
//!    `Err(ScheduleError::Full)` surfaced to the caller is deferred
//!    until a real fan-out regression appears in practice. The error
//!    type is reserved now so the surface does not change when the
//!    bound is added.
//! 6. **Main-thread only.** Only the main thread drains. Background
//!    threads only enqueue. Drain is single-reader and does not need
//!    locking beyond the MPSC itself.

use core::error::Error;
use core::future::Future;

extern crate alloc;

use alloc::boxed::Box;

/// Main-thread dispatcher and async runtime abstraction.
///
/// Implementations of this trait own the async runtime that drives
/// background work and the payload channel that shuttles the results
/// back to the main thread. See the module docs for the invariants
/// every implementation must satisfy.
pub trait Scheduler {
    /// Error type returned by scheduler operations.
    type Error: Error + Send + Sync + 'static;

    /// Opaque weak reference to a reactive owner.
    ///
    /// Owned by `sigil-reactive`; the scheduler only sees it as an
    /// identity-plus-generation token it can ask "is this still
    /// live?" at drain time.
    type OwnerHandle: Clone + Send + Sync + 'static;

    /// Opaque handle to a spawned future. Dropping the handle does
    /// *not* cancel the task — cancellation is owner-disposal-based
    /// per invariant (4). The handle exists for diagnostics and for
    /// tests that assert a specific spawn ran.
    ///
    /// Only [`spawn`][Scheduler::spawn] produces one of these.
    /// [`enqueue_main`][Scheduler::enqueue_main] is fire-and-forget:
    /// it does not return a task handle because the main-thread
    /// callback does not map onto a tokio `JoinHandle` and faking one
    /// would lie about its completion status.
    type SpawnTask: Send + 'static;

    /// Enqueue a callback to run on the main thread at the start of
    /// the next frame, scoped to the given owner.
    ///
    /// If the owner has been disposed by the time the dispatcher
    /// drains the queue, the callback is dropped silently — no panic,
    /// no error returned here. That is invariant (4).
    ///
    /// This is fire-and-forget from the caller's perspective. Success
    /// means the callback was successfully pushed onto the queue;
    /// actual execution happens later, on the main thread, at the
    /// next [`drain`][Scheduler::drain]. The enqueue itself only
    /// fails when the underlying channel has been torn down (the
    /// waking `App` is gone), in which case the caller's closure is
    /// dropped and `Err` is returned. Callers that need to observe
    /// the callback actually ran should use their own synchronisation
    /// (atomic flag, channel, signal write) inside the callback body.
    /// # Errors
    ///
    /// Returns an error when the main-thread callback queue is no
    /// longer available or refuses the payload.
    fn enqueue_main(
        &self,
        owner: Self::OwnerHandle,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), Self::Error>;

    /// Spawn a future on the async runtime, scoped to the given owner.
    ///
    /// The future's cancellation is tied to `owner` disposal: when the
    /// owner disposes, the spawned future is aborted synchronously
    /// (via an `on_cleanup` hook registered at spawn time) and its
    /// drop impl runs on the runtime worker thread. This is how
    /// `spawn_to_signal` cancellation from DR-007 is implemented.
    ///
    /// Background futures must only touch reactive state through
    /// `enqueue_main` or `dispatch_to_owner`; direct reactive reads
    /// and writes from a non-main thread will panic (DR-005 §4).
    ///
    /// # Main-thread requirement
    ///
    /// Must be called on the thread that owns the reactive runtime.
    /// The implementation registers an `on_cleanup` against `owner`
    /// so that disposing the owner aborts the future immediately; the
    /// cleanup registration touches the thread-local reactive arena
    /// and therefore cannot run on a background thread.
    ///
    /// # Stale-owner behaviour
    ///
    /// If `owner` is already disposed by the time `spawn` is called,
    /// the implementation must *not* run `future`. Swallowing the
    /// future silently is worse than the disposal-doesn't-cancel
    /// bug it replaces; implementations should drop the future
    /// unpolled and return a `SpawnTask` that reports completion
    /// (`JoinHandle::is_finished() == true`) immediately so callers
    /// can detect the stale-owner case.
    fn spawn<F>(&self, owner: Self::OwnerHandle, future: F) -> Self::SpawnTask
    where
        F: Future<Output = ()> + Send + 'static;

    /// Drain any queued main-thread callbacks.
    ///
    /// Called by `sigil-shell` at the start of each frame. For each
    /// drained callback, the dispatcher checks the owner's liveness;
    /// live-owner callbacks run on the main thread in enqueue order,
    /// dead-owner callbacks are dropped.
    ///
    /// Returns the number of callbacks actually executed (not counting
    /// dropped dead-owner callbacks). The count is informational —
    /// callers should not depend on it for correctness, only for
    /// metrics and tests.
    fn drain(&mut self) -> usize;
}
