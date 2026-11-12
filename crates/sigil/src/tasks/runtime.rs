use std::collections::HashMap;
use std::fmt;
#[cfg(feature = "kittest")]
use std::future::Future;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::Error;
use crate::reactive::Owner;
use crate::tasks::backend::{Backend, TaskWorkers};
use crate::tasks::task::TaskHandle;

pub(crate) struct DispatchMessage {
    pub(crate) owner: Option<Owner>,
    pub(crate) callback: Box<dyn FnOnce() + Send + 'static>,
}

#[derive(Clone)]
pub(crate) struct Dispatcher {
    pub(crate) tx: UnboundedSender<DispatchMessage>,
    pub(crate) waker: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl fmt::Debug for Dispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dispatcher").finish_non_exhaustive()
    }
}

impl Dispatcher {
    pub(crate) fn send(&self, message: DispatchMessage) -> Result<(), Error> {
        self.tx.send(message).map_err(|_| {
            Error::fixed(crate::ErrorKind::Closed, "main-thread dispatcher is closed")
        })?;
        (self.waker)();
        Ok(())
    }
}

const _: fn() = || {
    const fn assert_send_sync_clone_static<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync_clone_static::<Dispatcher>();
};

type TaskRegistry = Mutex<HashMap<Owner, Vec<TaskHandle>>>;

#[derive(Clone)]
pub(crate) struct SpawnParts {
    pub(crate) dispatcher: Dispatcher,
    pub(crate) backend: Backend,
    owner_tasks: Arc<TaskRegistry>,
}

/// The runtime that runs tasks and delivers their results to the UI thread.
pub struct AsyncRuntime {
    parts: SpawnParts,
    rx: UnboundedReceiver<DispatchMessage>,
}

impl fmt::Debug for AsyncRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncRuntime").finish_non_exhaustive()
    }
}

impl Drop for AsyncRuntime {
    fn drop(&mut self) {
        for (_, handles) in lock(&self.parts.owner_tasks).drain() {
            for handle in handles {
                handle.abort();
            }
        }
        crate::tasks::current::uninstall(&self.parts.dispatcher);
        #[cfg(not(target_arch = "wasm32"))]
        self.parts.backend.shutdown();
    }
}

fn lock(registry: &TaskRegistry) -> std::sync::MutexGuard<'_, HashMap<Owner, Vec<TaskHandle>>> {
    registry
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl AsyncRuntime {
    /// Creates a task runtime with the automatic worker count; it always returns `Ok`.
    pub fn new() -> Result<Self, crate::Error> {
        Ok(Self::with_workers(TaskWorkers::Auto))
    }

    pub(crate) fn with_workers(workers: TaskWorkers) -> Self {
        let backend = Backend::new(workers);
        let (tx, rx) = unbounded_channel();
        Self {
            parts: SpawnParts {
                dispatcher: Dispatcher {
                    tx,
                    waker: Arc::new(|| {}),
                },
                backend,
                owner_tasks: Arc::new(Mutex::new(HashMap::new())),
            },
            rx,
        }
    }

    /// Sets the function that wakes the UI thread.
    pub fn waker<F>(&mut self, waker: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.parts.dispatcher.waker = Arc::new(waker);
    }

    /// Runs waiting UI-thread work and returns how many items ran.
    pub fn pump(&mut self) -> usize {
        sweep_owner_tasks(&self.parts.owner_tasks);

        let budget = self.rx.len();
        let mut ran = 0_usize;
        for _ in 0..budget {
            let Ok(msg) = self.rx.try_recv() else {
                break;
            };
            let live = if let Some(owner) = msg.owner {
                owner.is_alive() && crate::reactive::owner_by_id(owner, msg.callback)
            } else {
                (msg.callback)();
                true
            };
            ran += usize::from(live);
        }
        crate::reactive::run_dispatch_deferred();
        ran
    }

    #[cfg(test)]
    pub(crate) fn spawn_parts(&self) -> SpawnParts {
        self.parts.clone()
    }

    /// Makes this runtime the current one on this thread.
    pub fn install_current(&self) {
        crate::tasks::current::install(&self.parts);
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn runtime_started(&self) -> bool {
        self.parts.backend.started()
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.rx.len()
    }
}

pub(crate) fn attach_task(parts: &SpawnParts, owner: Owner, task: TaskHandle) {
    lock(&parts.owner_tasks)
        .entry(owner)
        .or_default()
        .push(task.clone());
    let registered = crate::reactive::owner_by_id(owner, move || {
        crate::reactive::cleanup(move || task.abort());
    });
    if !registered {
        tracing::warn!(owner = ?owner, "task cleanup could not attach to a live owner");
    }
}

fn sweep_owner_tasks(registry: &TaskRegistry) {
    lock(registry).retain(|&owner_id, handles| {
        if !crate::reactive::is_live(owner_id) {
            for h in handles.drain(..) {
                h.abort();
            }
            return false;
        }
        handles.retain(|h| !h.is_finished());
        !handles.is_empty()
    });
}

impl AsyncRuntime {
    /// Queues a callback to run on the UI thread under an owner.
    #[cfg(feature = "kittest")]
    pub fn enqueue_main(
        &self,
        owner: Owner,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), Error> {
        self.parts.dispatcher.send(DispatchMessage {
            owner: Some(owner),
            callback: f,
        })
    }

    /// Spawns a future that is cancelled when its owner is disposed.
    #[cfg(feature = "kittest")]
    pub fn spawn<F>(&self, owner: Owner, future: F) -> TaskHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if !owner.is_alive() {
            drop(future);
            return TaskHandle::finished();
        }
        let task = self.parts.backend.spawn(owner, future, drop);
        attach_task(&self.parts, owner, task.clone());
        task
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "runtime_tests.rs"]
mod tests;
