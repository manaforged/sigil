use std::cell::RefCell;

use crate::reactive::Signal;

use crate::reactive::Owner;
use crate::tasks::dispatch::SignalWriter;
use crate::tasks::runtime::{Dispatcher, SpawnParts};

thread_local! {
    static HOST: RefCell<Option<SpawnParts>> = const { RefCell::new(None) };
}

pub(crate) fn install(parts: &SpawnParts) {
    HOST.with(|slot| {
        let mut host = slot.borrow_mut();
        if !current(host.as_ref(), &parts.dispatcher) {
            *host = Some(parts.clone());
        }
    });
}

fn current(host: Option<&SpawnParts>, dispatcher: &Dispatcher) -> bool {
    host.is_some_and(|parts| parts.dispatcher.tx.same_channel(&dispatcher.tx))
}

pub(crate) fn uninstall(dispatcher: &Dispatcher) {
    HOST.with(|slot| {
        let mut host = slot.borrow_mut();
        if current(host.as_ref(), dispatcher) {
            drop(host.take());
        }
    });
}

/// Tells whether a task dispatcher is installed on this thread.
#[cfg(feature = "kittest")]
#[must_use]
pub fn installed() -> bool {
    HOST.with(|slot| slot.borrow().is_some())
}

pub(crate) fn dispatcher() -> Option<Dispatcher> {
    HOST.with(|slot| slot.borrow().as_ref().map(|parts| parts.dispatcher.clone()))
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
    let parts = HOST.with(|slot| slot.borrow().clone());
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
