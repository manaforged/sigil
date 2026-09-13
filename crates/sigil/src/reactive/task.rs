use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::panic::Location;
use core::pin::Pin;
use core::task::{Context as StdContext, Poll};

use tokio::sync::oneshot;
use tokio::task::AbortHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskAwaitError {
    Cancelled,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityGone;

impl fmt::Display for EntityGone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("target entity (or its owner) has been disposed")
    }
}

impl std::error::Error for EntityGone {}

pub(crate) struct TaskInner<T> {
    pub(crate) abort: Option<AbortHandle>,
    pub(crate) result_rx: Option<oneshot::Receiver<T>>,
    pub(crate) caller: &'static Location<'static>,
}

#[must_use = "tasks are cancelled when dropped; call .detach() to keep running"]
pub struct Task<T> {
    inner: TaskInner<T>,
    _not_send: PhantomData<*const ()>,
}

impl<T> Task<T> {
    #[track_caller]
    #[allow(
        dead_code,
        reason = "consumed by EntityCx::spawn in the same DR-048 landing — module landing order"
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

    pub fn detach(mut self) {
        self.inner.abort = None;
    }

    pub fn cancel(&mut self) {
        if let Some(h) = self.inner.abort.take() {
            h.abort();
        }
    }

    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.inner.result_rx.is_none()
    }

    #[allow(
        clippy::future_not_send,
        reason = "Task<T> is !Send by design (DR-048 §6 — lives on main thread); the returned future inherits"
    )]
    pub async fn await_or_cancel(mut self) -> Result<T, TaskAwaitError> {
        let Some(rx) = self.inner.result_rx.take() else {
            return Err(TaskAwaitError::Cancelled);
        };
        self.inner.abort = None;
        match rx.await {
            Ok(v) => Ok(v),
            Err(_recv_err) => Err(TaskAwaitError::Cancelled),
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(h) = self.inner.abort.take() {
            h.abort();
        }
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

impl<T> Unpin for Task<T> {}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> Poll<T> {
        let this = self.as_mut().get_mut();
        let caller = this.inner.caller;
        let Some(rx) = this.inner.result_rx.as_mut() else {
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

#[allow(
    clippy::panic,
    reason = "DR-048 §6 documents Future<Output = T> panicking on cancel; graceful path is await_or_cancel"
)]
fn panic_cancelled(caller: &'static Location<'static>) -> ! {
    panic!(
        "Task<T> awaited after cancellation (spawn site: {caller}); use await_or_cancel for graceful handling"
    )
}

const _: fn() = || {
    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}
    const fn assert_static<T: 'static>() {}
    assert_send::<EntityGone>();
    assert_sync::<EntityGone>();
    assert_static::<EntityGone>();
    assert_send::<TaskAwaitError>();
    assert_sync::<TaskAwaitError>();
    assert_static::<TaskAwaitError>();
};
