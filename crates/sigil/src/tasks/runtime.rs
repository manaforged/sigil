use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, Mutex};

use crate::hal::Scheduler;
use crate::reactive::OwnerId;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::task::{AbortHandle, JoinHandle};

use crate::tasks::handle::OwnerHandle;
use crate::tasks::schedule_error::ScheduleError;

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

pub struct AsyncRuntime {
    rt: Handle,
    _owned: Option<Runtime>,
    sender: Dispatcher,
    rx: UnboundedReceiver<DispatchMessage>,
    owner_tasks: Arc<Mutex<HashMap<OwnerId, Vec<AbortHandle>>>>,
}

impl fmt::Debug for AsyncRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncRuntime").finish_non_exhaustive()
    }
}

impl Drop for AsyncRuntime {
    fn drop(&mut self) {
        for (_, handles) in self
            .owner_tasks
            .lock()
            .expect("owner_tasks mutex poisoned by a panicking holder")
            .drain()
        {
            for handle in handles {
                handle.abort();
            }
        }
        crate::tasks::current::uninstall(&self.sender);
    }
}

fn noop_waker() -> Arc<dyn Fn() + Send + Sync + 'static> {
    Arc::new(|| {})
}

#[derive(Clone)]
pub(crate) struct SpawnParts {
    pub(crate) dispatcher: Dispatcher,
    pub(crate) tokio: Handle,
    pub(crate) owner_tasks: Arc<Mutex<HashMap<OwnerId, Vec<AbortHandle>>>>,
}

impl AsyncRuntime {
    pub fn new() -> std::io::Result<Self> {
        let (rt, owned) = match Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => (handle, None),
            _ => {
                #[cfg(not(target_arch = "wasm32"))]
                let runtime = Builder::new_multi_thread().enable_all().build()?;
                #[cfg(target_arch = "wasm32")]
                let runtime = Builder::new_current_thread().enable_all().build()?;
                (runtime.handle().clone(), Some(runtime))
            }
        };
        let (tx, rx) = unbounded_channel();
        Ok(Self {
            rt,
            _owned: owned,
            sender: Dispatcher {
                tx,
                waker: noop_waker(),
            },
            rx,
            owner_tasks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn set_waker<F>(&mut self, waker: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.sender.waker = Arc::new(waker);
    }

    pub fn pump(&mut self) -> usize {
        Scheduler::drain(self)
    }

    #[must_use]
    pub fn dispatcher(&self) -> Dispatcher {
        self.sender.clone()
    }

    pub(crate) fn owner_tasks(&self) -> Arc<Mutex<HashMap<OwnerId, Vec<AbortHandle>>>> {
        Arc::clone(&self.owner_tasks)
    }

    pub(crate) fn tokio_handle(&self) -> Handle {
        self.rt.clone()
    }

    pub(crate) fn spawn_parts(&self) -> SpawnParts {
        SpawnParts {
            dispatcher: self.dispatcher(),
            tokio: self.tokio_handle(),
            owner_tasks: self.owner_tasks(),
        }
    }

    pub fn install_current(&self) {
        crate::tasks::current::install(self.spawn_parts());
    }

    #[must_use]
    #[doc(hidden)]
    pub fn pending_count(&self) -> usize {
        self.rx.len()
    }
}

pub(crate) fn register_abort(
    registry: &Mutex<HashMap<OwnerId, Vec<AbortHandle>>>,
    owner: OwnerId,
    abort: AbortHandle,
) {
    let mut map = registry
        .lock()
        .expect("owner_tasks mutex poisoned by a panicking holder");
    map.entry(owner).or_default().push(abort);
}

pub(crate) fn sweep_owner_tasks(registry: &Mutex<HashMap<OwnerId, Vec<AbortHandle>>>) {
    let mut map = registry
        .lock()
        .expect("owner_tasks mutex poisoned by a panicking holder");
    map.retain(|&owner_id, handles| {
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
    type SpawnTask = JoinHandle<()>;

    fn enqueue_main(
        &self,
        owner: Self::OwnerHandle,
        f: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), Self::Error> {
        self.sender.send(DispatchMessage { owner, callback: f })
    }

    fn spawn<F>(&self, owner: Self::OwnerHandle, future: F) -> Self::SpawnTask
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if !owner.is_alive() {
            drop(future);
            return self.rt.spawn(async {});
        }

        let join = self.rt.spawn(future);
        let abort = join.abort_handle();

        register_abort(&self.owner_tasks, owner.id(), abort.clone());

        let registered = crate::reactive::owner_by_id(owner.id(), move || {
            crate::reactive::on_cleanup(move || {
                abort.abort();
            });
        });
        if !registered {
            tracing::warn!(owner = ?owner.id(), "task cleanup could not attach to a live owner");
        }

        join
    }

    fn drain(&mut self) -> usize {
        sweep_owner_tasks(&self.owner_tasks);

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
        ran
    }
}
