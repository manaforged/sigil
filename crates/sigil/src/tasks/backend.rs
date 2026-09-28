use std::future::Future;

use crate::reactive::OwnerId;
use crate::tasks::task::{Guarded, Outcome, TaskHandle};

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskWorkers {
    #[default]
    Auto,
    Fixed(std::num::NonZeroUsize),
}

#[cfg(not(target_arch = "wasm32"))]
enum Pool {
    Idle,
    Running(tokio::runtime::Runtime),
    Closed,
}

#[cfg(not(target_arch = "wasm32"))]
struct Lazy {
    workers: TaskWorkers,
    handle: OnceLock<tokio::runtime::Handle>,
    pool: Mutex<Pool>,
}

#[derive(Clone)]
pub(crate) struct Backend {
    #[cfg(not(target_arch = "wasm32"))]
    lazy: Arc<Lazy>,
}

impl Backend {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new(workers: TaskWorkers) -> Self {
        use tokio::runtime::{Handle, RuntimeFlavor};
        let handle = OnceLock::new();
        if let Ok(current) = Handle::try_current()
            && current.runtime_flavor() == RuntimeFlavor::MultiThread
        {
            let _ = handle.set(current);
        }
        Self {
            lazy: Arc::new(Lazy {
                workers,
                handle,
                pool: Mutex::new(Pool::Idle),
            }),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) const fn new(_workers: TaskWorkers) -> Self {
        Self {}
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn handle(&self) -> Option<tokio::runtime::Handle> {
        if let Some(handle) = self.lazy.handle.get() {
            return Some(handle.clone());
        }
        let mut pool = self
            .lazy
            .pool
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(handle) = self.lazy.handle.get() {
            return Some(handle.clone());
        }
        if !matches!(*pool, Pool::Idle) {
            return None;
        }
        let runtime = match build(self.lazy.workers) {
            Ok(runtime) => runtime,
            Err(error) => {
                tracing::warn!(%error, "task runtime failed to start");
                return None;
            }
        };
        let handle = runtime.handle().clone();
        let _ = self.lazy.handle.set(handle.clone());
        *pool = Pool::Running(runtime);
        drop(pool);
        Some(handle)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn shutdown(&self) {
        let mut pool = self
            .lazy
            .pool
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = std::mem::replace(&mut *pool, Pool::Closed);
        drop(pool);
        if let Pool::Running(runtime) = previous {
            drop(runtime);
        }
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn started(&self) -> bool {
        self.lazy.handle.get().is_some()
    }

    pub(crate) fn spawn<T, F, C>(&self, owner: OwnerId, future: F, on_done: C) -> TaskHandle
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        C: FnOnce(Outcome<T>) + Send + 'static,
    {
        let task = TaskHandle::new();
        let done = task.clone();
        self.run(task.guard(future), move |outcome| {
            if let Outcome::Panicked(message) = &outcome {
                tracing::warn!(owner = ?owner, %message, "spawned task panicked");
            }
            done.finish();
            on_done(outcome);
        });
        task
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run<T, F, C>(&self, future: Guarded<F>, on_done: C)
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        C: FnOnce(Outcome<T>) + Send + 'static,
    {
        let Some(handle) = self.handle() else {
            drop(future);
            on_done(Outcome::Cancelled);
            return;
        };
        let work = handle.spawn(future);
        drop(handle.spawn(async move {
            let outcome = match work.await {
                Ok(Some(value)) => Outcome::Done(value),
                Ok(None) => Outcome::Cancelled,
                Err(join) if join.is_cancelled() => Outcome::Cancelled,
                Err(join) => Outcome::Panicked(match join.try_into_panic() {
                    Ok(payload) => super::panic_message(&*payload).to_owned(),
                    Err(join) => join.to_string(),
                }),
            };
            on_done(outcome);
        }));
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(clippy::unused_self)]
    fn run<T, F, C>(&self, future: Guarded<F>, on_done: C)
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
        C: FnOnce(Outcome<T>) + Send + 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            on_done(future.await.map_or(Outcome::Cancelled, Outcome::Done));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build(workers: TaskWorkers) -> std::io::Result<tokio::runtime::Runtime> {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    if let TaskWorkers::Fixed(count) = workers {
        builder.worker_threads(count.get());
    }
    builder.enable_all().build()
}
