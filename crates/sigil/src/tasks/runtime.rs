use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, Mutex};

use crate::hal::Scheduler;
use crate::reactive::OwnerId;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::tasks::backend::{Backend, OwnedRuntime};
use crate::tasks::handle::OwnerHandle;
use crate::tasks::schedule_error::ScheduleError;
use crate::tasks::task::TaskHandle;

pub(crate) struct DispatchMessage {
    pub(crate) owner: OwnerHandle,
    pub(crate) callback: Box<dyn FnOnce() + Send + 'static>,
}

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
    pub(crate) fn send(&self, message: DispatchMessage) -> Result<(), ScheduleError> {
        self.tx.send(message).map_err(|_| ScheduleError::Closed)?;
        (self.waker)();
        Ok(())
    }
}

const _: fn() = || {
    const fn assert_send_sync_clone_static<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync_clone_static::<Dispatcher>();
};

type OwnerTasks = Arc<Mutex<HashMap<OwnerId, Vec<TaskHandle>>>>;

#[derive(Clone)]
pub(crate) struct SpawnParts {
    pub(crate) dispatcher: Dispatcher,
    pub(crate) backend: Backend,
    owner_tasks: OwnerTasks,
}

pub struct AsyncRuntime {
    parts: SpawnParts,
    _owned: OwnedRuntime,
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
    }
}

fn noop_waker() -> Arc<dyn Fn() + Send + Sync + 'static> {
    Arc::new(|| {})
}

fn lock(
    registry: &Mutex<HashMap<OwnerId, Vec<TaskHandle>>>,
) -> std::sync::MutexGuard<'_, HashMap<OwnerId, Vec<TaskHandle>>> {
    registry
        .lock()
        .expect("owner_tasks mutex poisoned by a panicking holder")
}

impl AsyncRuntime {
    pub fn new() -> std::io::Result<Self> {
        let (backend, owned) = Backend::new()?;
        let (tx, rx) = unbounded_channel();
        Ok(Self {
            parts: SpawnParts {
                dispatcher: Dispatcher {
                    tx,
                    waker: noop_waker(),
                },
                backend,
                owner_tasks: Arc::new(Mutex::new(HashMap::new())),
            },
            _owned: owned,
            rx,
        })
    }

    pub fn set_waker<F>(&mut self, waker: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.parts.dispatcher.waker = Arc::new(waker);
    }

    pub fn pump(&mut self) -> usize {
        Scheduler::drain(self)
    }

    #[must_use]
    pub fn dispatcher(&self) -> Dispatcher {
        self.parts.dispatcher.clone()
    }

    pub(crate) fn spawn_parts(&self) -> SpawnParts {
        self.parts.clone()
    }

    pub fn install_current(&self) {
        crate::tasks::current::install(self.spawn_parts());
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.rx.len()
    }
}

pub(crate) fn attach_task(parts: &SpawnParts, owner: OwnerHandle, task: TaskHandle) {
    lock(&parts.owner_tasks)
        .entry(owner.id())
        .or_default()
        .push(task.clone());
    let registered = crate::reactive::owner_by_id(owner.id(), move || {
        crate::reactive::on_cleanup(move || task.abort());
    });
    if !registered {
        tracing::warn!(owner = ?owner.id(), "task cleanup could not attach to a live owner");
    }
}

fn sweep_owner_tasks(registry: &Mutex<HashMap<OwnerId, Vec<TaskHandle>>>) {
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

impl Scheduler for AsyncRuntime {
    type Error = ScheduleError;
    type OwnerHandle = OwnerHandle;
    type SpawnTask = TaskHandle;

    fn enqueue_main(
        &self,
        owner: Self::OwnerHandle,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), Self::Error> {
        self.parts
            .dispatcher
            .send(DispatchMessage { owner, callback: f })
    }

    fn spawn<F>(&self, owner: Self::OwnerHandle, future: F) -> Self::SpawnTask
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if !owner.is_alive() {
            drop(future);
            return TaskHandle::finished();
        }
        let task = self.parts.backend.spawn(owner.id(), future, drop);
        attach_task(&self.parts, owner, task.clone());
        task
    }

    fn drain(&mut self) -> usize {
        sweep_owner_tasks(&self.parts.owner_tasks);

        let budget = self.rx.len();
        let mut ran = 0_usize;
        for _ in 0..budget {
            let Ok(msg) = self.rx.try_recv() else {
                break;
            };
            if msg.owner.is_alive() {
                let callback = msg.callback;
                let ran_inner = crate::reactive::owner_by_id(msg.owner.id(), callback);
                if ran_inner {
                    ran += 1;
                }
            }
        }
        crate::reactive::run_dispatch_deferred();
        ran
    }
}
