use std::future::Future;

use crate::reactive::{SendableSignal, Signal};

use crate::tasks::handle::OwnerHandle;
use crate::tasks::runtime::{DispatchMessage, Dispatcher, attach_task};
use crate::tasks::schedule_error::ScheduleError;
use crate::tasks::status::Status;
use crate::tasks::task::{Outcome, TaskHandle};

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

pub(crate) fn dispatch_to_owner<F>(dispatcher: &Dispatcher, handle: OwnerHandle, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let sent = dispatcher.send(DispatchMessage {
        owner: handle,
        callback: Box::new(f),
    });
    if sent.is_err() {
        tracing::warn!(owner = ?handle.id(), "dispatch dropped: dispatcher closed");
    }
}

pub(crate) fn spawn_into<T, E, F, D>(
    parts: &crate::tasks::runtime::SpawnParts,
    owner: OwnerHandle,
    future: F,
    deliver: D,
) -> Option<TaskHandle>
where
    T: Send + 'static,
    E: Send + 'static,
    F: Future<Output = Result<T, E>> + Send + 'static,
    D: FnOnce(Status<T, E>) + Send + 'static,
{
    if !owner.is_alive() {
        drop(future);
        return None;
    }
    let dispatcher = parts.dispatcher.clone();
    let task = parts.backend.spawn(owner.id(), future, move |outcome| {
        let status = match outcome {
            Outcome::Done(Ok(value)) => Status::Ok(value),
            Outcome::Done(Err(error)) => Status::Err(error),
            Outcome::Panicked(message) => Status::Panicked(message),
            Outcome::Cancelled => return,
        };
        let sent = dispatcher.send(DispatchMessage {
            owner,
            callback: Box::new(move || deliver(status)),
        });
        if sent.is_err() {
            tracing::warn!(owner = ?owner.id(), "task result dropped: dispatcher closed");
        }
    });
    attach_task(parts, owner, task.clone());
    Some(task)
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
