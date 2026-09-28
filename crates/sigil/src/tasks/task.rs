use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

#[derive(Debug, Default)]
struct TaskState {
    aborted: AtomicBool,
    finished: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

#[derive(Clone, Debug)]
pub struct TaskHandle {
    state: Arc<TaskState>,
}

impl TaskHandle {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(TaskState::default()),
        }
    }

    pub(crate) fn finished() -> Self {
        let handle = Self::new();
        handle.finish();
        handle
    }

    pub fn abort(&self) {
        self.state.aborted.store(true, Ordering::SeqCst);
        let waker = self.waker().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.state.finished.load(Ordering::SeqCst)
    }

    pub(crate) fn finish(&self) {
        self.state.finished.store(true, Ordering::SeqCst);
    }

    pub(crate) fn guard<F: Future>(&self, future: F) -> Guarded<F> {
        Guarded {
            handle: self.clone(),
            future: Box::pin(future),
        }
    }

    fn aborted(&self) -> bool {
        self.state.aborted.load(Ordering::SeqCst)
    }

    fn waker(&self) -> MutexGuard<'_, Option<Waker>> {
        self.state
            .waker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

pub(crate) struct Guarded<F> {
    handle: TaskHandle,
    future: Pin<Box<F>>,
}

impl<F: Future> Future for Guarded<F> {
    type Output = Option<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.handle.aborted() {
            return Poll::Ready(None);
        }
        *this.handle.waker() = Some(cx.waker().clone());
        if this.handle.aborted() {
            return Poll::Ready(None);
        }
        this.future.as_mut().poll(cx).map(Some)
    }
}

pub(crate) enum Outcome<T> {
    Done(T),
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Panicked(String),
    Cancelled,
}
