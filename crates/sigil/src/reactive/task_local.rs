use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::panic::Location;
use core::pin::Pin;
use core::task::{Context as StdContext, Poll};

use tokio::sync::oneshot;
use tokio::task::AbortHandle;

use crate::reactive::task::TaskAwaitError;

#[must_use = "tasks are cancelled when dropped; call .detach() to keep running"]
pub struct LocalTask<R> {
    abort: Option<AbortHandle>,
    result_rx: Option<oneshot::Receiver<R>>,
    caller: &'static Location<'static>,
    _not_send: PhantomData<*const ()>,
}

impl<R> LocalTask<R> {
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

    pub fn detach(mut self) {
        self.abort = None;
    }

    pub fn cancel(&mut self) {
        if let Some(h) = self.abort.take() {
            h.abort();
        }
    }

    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.result_rx.is_none()
    }

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

impl<R> Unpin for LocalTask<R> {}

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

#[allow(
    clippy::panic,
    reason = "DR-048 §6 documents Future<Output = R> panicking on cancel; graceful path is await_or_cancel"
)]
fn panic_cancelled(caller: &'static Location<'static>) -> ! {
    panic!(
        "LocalTask<R> awaited after cancellation (spawn site: {caller}); use await_or_cancel for graceful handling"
    )
}
