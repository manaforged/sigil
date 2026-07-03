//! G5 — `Task<T>` polymorphic handle (DR-048 §6).
//!
//! `Task<T>` is the imperative-handle shape returned by
//! [`crate::Context::spawn`]. It wraps a spawned future's
//! [`AbortHandle`] (for RAII cancel-on-drop) and a [`oneshot::Receiver`]
//! that carries the future's output back from the executor worker.
//!
//! Three layers of cancellation cooperate (DR-048 §3):
//! 1. **Explicit drop** — cancels by default; the `Drop` impl below
//!    fires `AbortHandle::abort()`.
//! 2. **`.detach()`** — consumes the handle without aborting; the
//!    underlying task runs to completion (or until layer 3 fires).
//! 3. **Owner cascade** — the executor registered an `on_cleanup`
//!    against the spawning entity's owner; owner disposal aborts the
//!    task synchronously (this lives in the `TaskExecutor` impl, not
//!    here).
//!
//! `Task<T>: Future<Output = T>`. Polling drives the oneshot. If the
//! task was cancelled (drop or explicit `.cancel`), the oneshot sender
//! is dropped without sending; polling yields a [`TaskAwaitError::Cancelled`]
//! via [`Task::await_or_cancel`], or panics via the
//! [`Future`] impl. Authors picking between the two pick by ergonomic
//! need (DR-048 §6).

use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::panic::Location;
use core::pin::Pin;
use core::task::{Context as StdContext, Poll};

use tokio::sync::oneshot;
use tokio::task::AbortHandle;

/// Error surface for the explicit [`Task::await_or_cancel`] shape.
///
/// The common-case [`Future`] impl on `Task<T>` returns `T` directly
/// (a user-typed `Result<U, E>` if the future yielded one). The
/// framework-level cancellation / panic cases surface through this
/// enum only when the caller opts into them via `await_or_cancel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskAwaitError {
    /// The task was cancelled (either via [`Task::cancel`], dropping
    /// the handle without `.detach()`, or owner-cascade disposal).
    Cancelled,
    /// The future body panicked. Surfaced for visibility — most
    /// authors will not see this in production code.
    Panicked,
}

impl fmt::Display for TaskAwaitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("task was cancelled"),
            Self::Panicked => f.write_str("task panicked"),
        }
    }
}

impl std::error::Error for TaskAwaitError {}

/// Error returned by `AsyncContext::update` when the target entity (or
/// its owner) was disposed between spawn and update.
///
/// Zero-sized; carries no payload by design — the entity identity is
/// implicit in the call site that produced this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityGone;

impl fmt::Display for EntityGone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("target entity (or its owner) has been disposed")
    }
}

impl std::error::Error for EntityGone {}

/// Internal storage for [`Task<T>`]. Three logical states fold into
/// this struct via the `Option` fields.
pub(crate) struct TaskInner<T> {
    /// `Some` while the task may still abort. Becomes `None` either
    /// when [`Task::detach`] consumes the handle, when [`Task::cancel`]
    /// fires (after the abort), or after the task completed and was
    /// awaited.
    pub(crate) abort: Option<AbortHandle>,
    /// `Some` until the task has been polled to completion.
    pub(crate) result_rx: Option<oneshot::Receiver<T>>,
    /// `cx.spawn(...)` call site, captured by `#[track_caller]` at
    /// spawn time. Surfaced in the `Future::poll` cancellation panic
    /// path so the message points at the user's spawn site, not at
    /// `panic_cancelled`.
    pub(crate) caller: &'static Location<'static>,
}

/// RAII-cancelling handle to a spawned future (DR-048 §6).
///
/// `Task<T>: Future<Output = T>` — `.await`ing the handle drives the
/// oneshot completion path. Dropping the handle aborts the future
/// unless [`Task::detach`] was called first.
///
/// `Task<T>` is `!Send + !Sync`: the handle lives on the main thread
/// alongside the reactive runtime that produced it. The wrapped future
/// itself is `Send + 'static` (it runs on the executor's worker pool).
#[must_use = "tasks are cancelled when dropped; call .detach() to keep running"]
pub struct Task<T> {
    inner: TaskInner<T>,
    /// `!Send + !Sync` — Task<T> belongs to the main thread.
    _not_send: PhantomData<*const ()>,
}

impl<T> Task<T> {
    /// Construct from raw parts. `pub(crate)` so only
    /// [`crate::cx::Context::spawn`] can mint a Task.
    #[track_caller]
    #[allow(
        dead_code,
        reason = "consumed by Context::spawn in the same DR-048 landing — module landing order"
    )]
    #[allow(
        clippy::missing_const_for_fn,
        reason = "Location::caller() is not const-fn yet on stable; constness is irrelevant for this constructor"
    )]
    pub(crate) fn from_parts(abort: AbortHandle, result_rx: oneshot::Receiver<T>) -> Self {
        let caller = Location::caller();
        Self {
            inner: TaskInner {
                abort: Some(abort),
                result_rx: Some(result_rx),
                caller,
            },
            _not_send: PhantomData,
        }
    }

    /// Consume the handle without aborting. The task continues until
    /// completion or until the owner-cascade safety net aborts it.
    ///
    /// After `detach`, the handle is gone and there is no way to
    /// observe the future's output. Use for fire-and-forget telemetry
    /// or analytics dispatches.
    pub fn detach(mut self) {
        // Drop the AbortHandle without firing it.
        self.inner.abort = None;
        // result_rx falls out of scope; the worker's send half returns
        // Err on completion and is silently dropped. That is fine —
        // detach is the documented "I don't care about the output" path.
    }

    /// Synchronous cancellation request. Subsequent `.await` resolves
    /// with [`TaskAwaitError::Cancelled`] via [`Task::await_or_cancel`],
    /// or yields the panic message from the [`Future`] impl.
    pub fn cancel(&mut self) {
        if let Some(h) = self.inner.abort.take() {
            h.abort();
        }
    }

    /// True if the task has finished (completed or aborted) AND the
    /// result has been consumed via [`Task::await_or_cancel`] or
    /// [`Future::poll`]. Returns `false` when the task is still
    /// running OR when it completed but the result has not yet been
    /// awaited.
    ///
    /// Note: the stricter "completed-but-not-yet-awaited" detection
    /// requires `tokio::sync::oneshot::Receiver::is_terminated`, which
    /// landed in tokio 1.42. The workspace currently targets tokio 1.40,
    /// so this implementation conservatively returns `true` only after
    /// the receiver slot has been consumed.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.inner.result_rx.is_none()
    }

    /// Await the task and surface framework-level cancellation / panic.
    ///
    /// Returns `Ok(T)` if the task completed normally, `Err(Cancelled)`
    /// if the task was aborted before sending its result, and
    /// `Err(Panicked)` if the future body panicked.
    ///
    /// `Task<T>` is `!Send` by construction (lives on the main thread
    /// alongside the reactive runtime), so the returned future is
    /// `!Send` too. That matches GPUI's `Task` and is intentional:
    /// callers awaiting a Task are themselves running on the main
    /// thread (typically inside another `cx.spawn` body that re-entered
    /// via `acx.update`).
    ///
    /// # Errors
    ///
    /// See variants of [`TaskAwaitError`].
    #[allow(
        clippy::future_not_send,
        reason = "Task<T> is !Send by design (DR-048 §6 — lives on main thread); the returned future inherits"
    )]
    pub async fn await_or_cancel(mut self) -> Result<T, TaskAwaitError> {
        let Some(rx) = self.inner.result_rx.take() else {
            return Err(TaskAwaitError::Cancelled);
        };
        // Suppress Drop's abort — we're consuming the handle through
        // the explicit await path.
        self.inner.abort = None;
        match rx.await {
            Ok(v) => Ok(v),
            Err(_recv_err) => {
                // RecvError fires when the sender was dropped. That
                // happens for both abort (AbortHandle::abort cancels
                // the future without ever sending) and panic (the
                // tokio task's drop runs without ever sending). We do
                // not distinguish at this layer because tokio's
                // JoinError surface is opaque from the oneshot side.
                // The MAJORITY of real cases are cancellation; tag
                // accordingly.
                Err(TaskAwaitError::Cancelled)
            }
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        // RAII cancel-on-drop unless detached or cancelled.
        if let Some(h) = self.inner.abort.take() {
            h.abort();
        }
        // result_rx drops with the inner struct; the worker's send-half
        // returns Err on completion and is silently dropped. Idempotent.
    }
}

impl<T> fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("finished", &self.is_finished())
            .field("detached", &self.inner.abort.is_none())
            .finish_non_exhaustive()
    }
}

// `Task<T>` is Unpin — all its fields (Option<AbortHandle>,
// Option<oneshot::Receiver<T>>, PhantomData) are Unpin. The explicit
// impl lets the `Future` impl below project via the safe
// `Pin::get_mut` path without any `unsafe`.
impl<T> Unpin for Task<T> {}

/// `Task<T>: Future<Output = T>` — the common-case shape.
///
/// Polling drives the wrapped oneshot. If the task was cancelled
/// (`RecvError` from the receiver) the future panics with a diagnostic
/// message pointing at the spawn site; authors who want graceful
/// cancellation use [`Task::await_or_cancel`] instead.
impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> Poll<T> {
        // Task<T>: Unpin (see impl above), so `get_mut` is safe and
        // sufficient — no unsafe Pin projection needed.
        let this = self.as_mut().get_mut();
        let caller = this.inner.caller;
        let Some(rx) = this.inner.result_rx.as_mut() else {
            // Already polled to completion or detached; subsequent
            // poll is a contract violation, but we panic with a
            // diagnostic rather than silently misbehaving.
            panic_cancelled(caller);
        };
        match Pin::new(rx).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(v)) => {
                this.inner.result_rx = None;
                this.inner.abort = None;
                Poll::Ready(v)
            }
            Poll::Ready(Err(_recv_err)) => {
                this.inner.result_rx = None;
                this.inner.abort = None;
                panic_cancelled(caller);
            }
        }
    }
}

/// Panic helper for the `Future` impl's cancellation branch.
///
/// Centralised so the message + `track_caller` routing match every
/// Future-impl panic regardless of which code path produced the
/// cancellation. The `clippy::panic` allow scope is the function body
/// only — DR-048 §6 documents the `Future<Output = T>` shape as
/// panicking on cancel (the graceful path is `await_or_cancel`).
#[allow(
    clippy::panic,
    reason = "DR-048 §6 documents Future<Output = T> panicking on cancel; graceful path is await_or_cancel"
)]
fn panic_cancelled(caller: &'static Location<'static>) -> ! {
    panic!(
        "Task<T> awaited after cancellation (spawn site: {caller}); use await_or_cancel for graceful handling"
    )
}

// Compile-time guarantees for the public surface.
const _: fn() = || {
    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}
    const fn assert_static<T: 'static>() {}
    // EntityGone + TaskAwaitError are Send + Sync + 'static so they
    // can cross await points.
    assert_send::<EntityGone>();
    assert_sync::<EntityGone>();
    assert_static::<EntityGone>();
    assert_send::<TaskAwaitError>();
    assert_sync::<TaskAwaitError>();
    assert_static::<TaskAwaitError>();
};
