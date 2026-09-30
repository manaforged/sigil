use std::cell::RefCell;

use crate::reactive::Signal;

use crate::tasks::dispatch::SignalWriter;
use crate::tasks::handle::OwnerHandle;
use crate::tasks::runtime::{Dispatcher, SpawnParts};

thread_local! {
    static HOST: RefCell<Option<SpawnParts>> = const { RefCell::new(None) };
}

pub(crate) fn install(parts: SpawnParts) {
    HOST.with(|slot| *slot.borrow_mut() = Some(parts));
}

pub(crate) fn uninstall(dispatcher: &Dispatcher) {
    HOST.with(|slot| {
        let mut host = slot.borrow_mut();
        if host
            .as_ref()
            .is_some_and(|parts| parts.dispatcher.tx.same_channel(&dispatcher.tx))
        {
            drop(host.take());
        }
    });
}

#[must_use]
pub fn installed() -> bool {
    HOST.with(|slot| slot.borrow().is_some())
}

pub(crate) fn dispatcher() -> Option<Dispatcher> {
    HOST.with(|slot| slot.borrow().as_ref().map(|parts| parts.dispatcher.clone()))
}

pub(crate) fn scope_parts(caller: &'static str) -> Option<(OwnerHandle, SpawnParts)> {
    let Some(owner) = OwnerHandle::current() else {
        tracing::warn!(target: "sigil::tasks", caller, "called outside a reactive scope; dropped the work");
        return None;
    };
    let parts = HOST.with(|slot| slot.borrow().clone());
    if parts.is_none() {
        tracing::warn!(
            target: "sigil::tasks",
            caller,
            "called before AsyncRuntime::install_current; dropped the work"
        );
    }
    parts.map(|parts| (owner, parts))
}

#[must_use]
#[track_caller]
pub fn signal_writer<T>(signal: Signal<T>) -> SignalWriter<T>
where
    T: Send + 'static,
{
    let owner = OwnerHandle::current().expect("signal_writer() called outside a reactive scope");
    let dispatcher = dispatcher().expect("signal_writer() requires AsyncRuntime::install_current");
    SignalWriter::new(dispatcher, owner, signal)
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

#[must_use]
#[track_caller]
pub fn spawn<T, E, F>(future: F) -> Signal<crate::tasks::Status<T, E>>
where
    T: Send + 'static,
    E: Send + 'static,
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let status = crate::reactive::signal(crate::tasks::Status::Pending);
    if let Some((owner, parts)) = scope_parts("spawn") {
        let target = status.into_sendable();
        let _handle = crate::tasks::dispatch::spawn_into(&parts, owner, future, move |done| {
            target.into_signal().try_set_always(done);
        });
    }
    status
}
