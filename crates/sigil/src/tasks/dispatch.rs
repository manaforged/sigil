use std::future::Future;

use crate::reactive::{SendableSignal, Signal};

use crate::Error;
use crate::reactive::Owner;
use crate::tasks::runtime::{DispatchMessage, Dispatcher, attach_task};
use crate::tasks::status::Status;
use crate::tasks::task::{Outcome, TaskHandle};

/// Write a signal from another thread.
#[derive(Clone)]
pub struct SignalWriter<T: Send + 'static> {
    dispatcher: Dispatcher,
    owner: Owner,
    signal: SendableSignal<T>,
}

impl<T: Send + 'static> SignalWriter<T> {
    pub(crate) fn new(dispatcher: Dispatcher, owner: Owner, signal: Signal<T>) -> Self {
        Self {
            dispatcher,
            owner,
            signal: signal.into_sendable(),
        }
    }

    /// Writes `value` to the signal from any thread.
    pub fn set(&self, value: T) -> Result<(), Error> {
        self.send(move |signal| signal.set_always(value))
    }

    /// Changes the signal's value in place from any thread.
    pub fn update<F>(&self, update: F) -> Result<(), Error>
    where
        F: FnOnce(&mut T) + Send + 'static,
    {
        self.send(move |signal| signal.update(update))
    }

    fn send(&self, write: impl FnOnce(Signal<T>) + Send + 'static) -> Result<(), Error> {
        let signal = self.signal;
        self.dispatcher.send(DispatchMessage {
            owner: self.owner,
            callback: Box::new(move || write(signal.into_signal())),
        })
    }
}

pub(crate) fn dispatch_to_owner<F>(dispatcher: &Dispatcher, handle: Owner, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let sent = dispatcher.send(DispatchMessage {
        owner: handle,
        callback: Box::new(f),
    });
    if sent.is_err() {
        tracing::warn!(owner = ?handle, "dispatch dropped: dispatcher closed");
    }
}

pub(crate) fn spawn_into<T, E, F, D>(
    parts: &crate::tasks::runtime::SpawnParts,
    owner: Owner,
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
    let task = parts.backend.spawn(owner, future, move |outcome| {
        let status = match outcome {
            Outcome::Done(Ok(value)) => Status::Ok(value),
            Outcome::Done(Err(error)) => Status::Err(error),
            Outcome::Panicked(message) => Status::Panicked(message),
            Outcome::Cancelled => return,
        };
        dispatch_to_owner(&dispatcher, owner, move || deliver(status));
    });
    attach_task(parts, owner, task.clone());
    Some(task)
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
