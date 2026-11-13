use std::cell::RefCell;

use crate::reactive::Signal;

use crate::reactive::Owner;
use crate::tasks::dispatch::SignalWriter;
use crate::tasks::runtime::{Dispatcher, SpawnParts};

thread_local! {
    static HOST: RefCell<Vec<SpawnParts>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn install(parts: &SpawnParts) {
    HOST.with(|slot| {
        let mut stack = slot.borrow_mut();
        if stack.last().is_some_and(|top| top.id == parts.id) {
            return;
        }
        stack.retain(|entry| entry.id != parts.id);
        stack.push(parts.clone());
    });
}

pub(crate) fn uninstall(id: u64) {
    HOST.with(|slot| slot.borrow_mut().retain(|entry| entry.id != id));
}

/// Tells whether a task dispatcher is installed on this thread.
#[cfg(feature = "kittest")]
#[must_use]
pub fn installed() -> bool {
    HOST.with(|slot| !slot.borrow().is_empty())
}

pub(crate) fn dispatcher() -> Option<Dispatcher> {
    HOST.with(|slot| slot.borrow().last().map(|parts| parts.dispatcher.clone()))
}

pub(crate) fn scope_parts(caller: &'static str) -> Option<(Owner, SpawnParts)> {
    let Some(owner) = Owner::current() else {
        tracing::warn!(target: "sigil::tasks", caller, "called outside a reactive scope; dropped the work");
        return None;
    };
    host_parts(caller).map(|parts| (owner, parts))
}

impl<T: Send + 'static> Signal<T> {
    /// Returns a handle that writes this signal from another thread.
    #[must_use]
    #[track_caller]
    pub fn writer(self) -> SignalWriter<T> {
        let dispatcher =
            dispatcher().expect("Signal::writer requires AsyncRuntime::install_current");
        SignalWriter::new(dispatcher, self)
    }
}

pub(crate) fn host_parts(caller: &'static str) -> Option<SpawnParts> {
    let parts = HOST.with(|slot| slot.borrow().last().cloned());
    if parts.is_none() {
        tracing::warn!(
            target: "sigil::tasks",
            caller,
            "called before AsyncRuntime::install_current; dropped the work"
        );
    }
    parts
}

/// Starts async work that runs once.
#[must_use]
#[track_caller]
pub fn spawn<T, E, F>(future: F) -> Signal<crate::tasks::Status<T, E>>
where
    T: Send + 'static,
    E: Send + 'static,
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let status = crate::reactive::signal(crate::tasks::Status::Pending);
    let started = scope_parts("spawn").and_then(|(owner, parts)| {
        let target = status.into_sendable();
        crate::tasks::dispatch::spawn_into(&parts, owner, future, move |done| {
            target.into_signal().try_set_always(done);
        })
    });
    if started.is_none() {
        status.update(|state| *state = crate::tasks::Status::Idle);
    }
    status
}
