//! `LocalTask<R>` — the `!Send` sibling of [`crate::Task`] (DR-048 §6
//! amendment, W4.99.B3).
//!
//! Identical RAII-cancel shape and `Future` impl to [`crate::Task`],
//! but the wrapped future is `!Send` (it ran on a
//! [`crate::LocalTaskExecutor`], typically a tokio `LocalSet`).
//! The `R` output also does not need to be `Send` — the result is
//! delivered through a `tokio::sync::oneshot` whose sender lives in the
//! future body on the same thread as the receiver in the handle.
//!
//! See [`crate::EntityCx::spawn_local`] for the producer side.

use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::panic::Location;
use core::pin::Pin;
use core::task::{Context as StdContext, Poll};

use tokio::sync::oneshot;
use tokio::task::AbortHandle;

use crate::task::TaskAwaitError;

/// RAII-cancelling handle to a `!Send` spawned future (DR-048 §6
/// amendment).
///
/// `LocalTask<R>: Future<Output = R>` — `.await`ing the handle drives
/// the oneshot completion path. Dropping the handle aborts the future
/// unless [`LocalTask::detach`] was called first.
///
/// `LocalTask<R>` is `!Send + !Sync` by construction: it holds a
/// `oneshot::Receiver<R>` whose `R` may be `!Send`, and it lives on the
/// main thread alongside the reactive runtime that produced it. The
/// `_not_send` phantom marker makes the negative bound robust to future
/// field changes.
#[must_use = "tasks are cancelled when dropped; call .detach() to keep running"]
pub struct LocalTask<R> {
    abort: Option<AbortHandle>,
    result_rx: Option<oneshot::Receiver<R>>,
    /// `cx.spawn_local(...)` call site, captured by `#[track_caller]`
    /// at spawn time. Surfaced in the `Future::poll` cancellation panic
    /// path so the message points at the user's spawn site.
    caller: &'static Location<'static>,
    /// `!Send + !Sync` — lives on the main thread.
    _not_send: PhantomData<*const ()>,
}

impl<R> LocalTask<R> {
    /// Construct from raw parts. `pub(crate)` so only
    /// [`crate::cx::EntityCx::spawn_local`] can mint a `LocalTask`.
    #[track_caller]
    #[allow(
        clippy::missing_const_for_fn,
        reason = "Location::caller() is not const-fn on stable; the constructor is shape-stable regardless"
    )]
    #[allow(
        dead_code,
        reason = "consumed by EntityCx::spawn_local in the same W4.99.B3 landing — module landing order"
    )]
    pub(crate) fn from_parts(abort: AbortHandle, result_rx: oneshot::Receiver<R>) -> Self {
        Self {
            abort: Some(abort),
            result_rx: Some(result_rx),
            caller: Location::caller(),
            _not_send: PhantomData,
        }
    }

    /// Consume the handle without aborting. The task continues until
    /// completion or until the owner-cascade safety net aborts it.
    pub fn detach(mut self) {
        self.abort = None;
    }

    /// Synchronous cancellation request. Subsequent `.await` resolves
    /// with [`TaskAwaitError::Cancelled`] via
    /// [`LocalTask::await_or_cancel`], or yields the panic message from
    /// the [`Future`] impl.
    pub fn cancel(&mut self) {
        if let Some(h) = self.abort.take() {
            h.abort();
        }
    }

    /// True if the task has finished (completed or aborted) AND the
    /// result has been consumed via [`LocalTask::await_or_cancel`] or
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
        self.result_rx.is_none()
    }

    /// Await the task and surface framework-level cancellation / panic.
    ///
    /// Returns `Ok(R)` on normal completion, `Err(Cancelled)` if the
    /// task was aborted before sending its result.
    ///
    /// # Errors
    ///
    /// See variants of [`TaskAwaitError`]. As with [`crate::Task`], we
    /// cannot distinguish abort from worker-panic at this layer; the
    /// majority of real cases are cancellation, so we tag accordingly.
    #[allow(
        clippy::future_not_send,
        reason = "LocalTask<R> is !Send by design (W4.99.B3 — drives !Send futures); the returned future inherits"
    )]
    pub async fn await_or_cancel(mut self) -> Result<R, TaskAwaitError> {
        let Some(rx) = self.result_rx.take() else {
            return Err(TaskAwaitError::Cancelled);
        };
        self.abort = None;
        match rx.await {
            Ok(v) => Ok(v),
            Err(_recv_err) => Err(TaskAwaitError::Cancelled),
        }
    }
}

impl<R> Drop for LocalTask<R> {
    fn drop(&mut self) {
        if let Some(h) = self.abort.take() {
            h.abort();
        }
    }
}

impl<R> fmt::Debug for LocalTask<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalTask")
            .field("finished", &self.is_finished())
            .field("detached", &self.abort.is_none())
            .finish_non_exhaustive()
    }
}

// `LocalTask<R>` is Unpin — all fields (Option<AbortHandle>,
// Option<oneshot::Receiver<R>>, &'static Location, PhantomData) are
// Unpin. The explicit impl lets the `Future` impl below project via
// the safe `Pin::get_mut` path without `unsafe`.
impl<R> Unpin for LocalTask<R> {}

/// `LocalTask<R>: Future<Output = R>` — same shape as [`crate::Task`].
///
/// Polling drives the wrapped oneshot. If the task was cancelled the
/// future panics with a diagnostic message pointing at the spawn site;
/// authors who want graceful cancellation use
/// [`LocalTask::await_or_cancel`] instead.
impl<R> Future for LocalTask<R> {
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> Poll<R> {
        let this = self.as_mut().get_mut();
        let caller = this.caller;
        let Some(rx) = this.result_rx.as_mut() else {
            panic_cancelled(caller);
        };
        match Pin::new(rx).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(v)) => {
                this.result_rx = None;
                this.abort = None;
                Poll::Ready(v)
            }
            Poll::Ready(Err(_recv_err)) => {
                this.result_rx = None;
                this.abort = None;
                panic_cancelled(caller);
            }
        }
    }
}

/// Panic helper for the `Future` impl's cancellation branch — matches
/// [`crate::task`]'s helper of the same name.
#[allow(
    clippy::panic,
    reason = "DR-048 §6 documents Future<Output = R> panicking on cancel; graceful path is await_or_cancel"
)]
fn panic_cancelled(caller: &'static Location<'static>) -> ! {
    panic!(
        "LocalTask<R> awaited after cancellation (spawn site: {caller}); use await_or_cancel for graceful handling"
    )
}
