use std::future::Future;

use crate::reactive::{SendableSignal, Signal, signal};

use crate::tasks::handle::OwnerHandle;
use crate::tasks::runtime::{AsyncRuntime, DispatchMessage, Dispatcher, register_abort};
use crate::tasks::schedule_error::ScheduleError;
use crate::tasks::status::Status;

#[derive(Clone)]
pub struct SignalWriter<T: Send + 'static> {
    dispatcher: Dispatcher,
    owner: OwnerHandle,
    signal: SendableSignal<T>,
}

impl<T: Send + 'static> SignalWriter<T> {
    pub(crate) fn new(dispatcher: Dispatcher, owner: OwnerHandle, signal: Signal<T>) -> Self {
        Self {
            dispatcher,
            owner,
            signal: signal.into_sendable(),
        }
    }

    pub fn set(&self, value: T) -> Result<(), ScheduleError> {
        let signal = self.signal;
        self.dispatcher.send(DispatchMessage {
            owner: self.owner,
            callback: Box::new(move || signal.into_signal().set_always(value)),
        })
    }

    pub fn update<F>(&self, update: F) -> Result<(), ScheduleError>
    where
        F: FnOnce(&mut T) + Send + 'static,
    {
        let signal = self.signal;
        self.dispatcher.send(DispatchMessage {
            owner: self.owner,
            callback: Box::new(move || signal.into_signal().update(update)),
        })
    }
}

pub fn dispatch_to_owner<F>(dispatcher: &Dispatcher, handle: OwnerHandle, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let _ = dispatcher.send(DispatchMessage {
        owner: handle,
        callback: Box::new(f),
    });
}

#[doc(hidden)]
#[track_caller]
pub fn spawn_to_signal<T, E, F>(
    runtime: &AsyncRuntime,
    owner: OwnerHandle,
    future: F,
) -> Signal<Status<T, E>>
where
    T: Send + 'static,
    E: Send + 'static,
    F: Future<Output = Result<T, E>> + Send + 'static,
{
    spawn_with(runtime.spawn_parts(), owner, future)
}

pub(crate) fn spawn_with<T, E, F>(
    parts: crate::tasks::runtime::SpawnParts,
    owner: OwnerHandle,
    future: F,
) -> Signal<Status<T, E>>
where
    T: Send + 'static,
    E: Send + 'static,
    F: Future<Output = Result<T, E>> + Send + 'static,
{
    let sig: Signal<Status<T, E>> = signal(Status::Pending);
    let sig_sendable = sig.into_sendable();
    if !owner.is_alive() {
        drop(future);
        return sig;
    }

    let dispatcher = parts.dispatcher;
    let join = parts.tokio.spawn(async move {
        let result = future.await;
        let _ = dispatcher.send(DispatchMessage {
            owner,
            callback: Box::new(move || {
                let sig_local = sig_sendable.into_signal();
                match result {
                    Ok(v) => sig_local.set_always(Status::Ok(v)),
                    Err(e) => sig_local.set_always(Status::Err(e)),
                }
            }),
        });
    });

    let abort = join.abort_handle();
    register_abort(&parts.owner_tasks, owner.id(), abort.clone());
    let _ = crate::reactive::owner_by_id(owner.id(), move || {
        crate::reactive::on_cleanup(move || {
            abort.abort();
        });
    });
    sig
}
