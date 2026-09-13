use std::cell::RefCell;
use std::future::Future;

use crate::reactive::Signal;

use crate::tasks::dispatch::SignalWriter;
use crate::tasks::dispatch::spawn_with;
use crate::tasks::handle::OwnerHandle;
use crate::tasks::runtime::{Dispatcher, SpawnParts};
use crate::tasks::status::Status;

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
pub fn dispatcher() -> Dispatcher {
    HOST.with(|slot| {
        slot.borrow()
            .as_ref()
            .expect("dispatcher() requires AsyncRuntime::install_current")
            .dispatcher
            .clone()
    })
}

#[track_caller]
pub fn post(f: impl FnOnce() + Send + 'static) {
    let owner = OwnerHandle::current().expect("post() called outside a reactive scope");
    HOST.with(|slot| {
        let parts = slot
            .borrow()
            .as_ref()
            .expect("post() requires AsyncRuntime::install_current")
            .clone();
        crate::tasks::dispatch::dispatch_to_owner(&parts.dispatcher, owner, f);
    });
}

#[must_use]
#[track_caller]
pub fn spawn<T, E, F>(future: F) -> Signal<Status<T, E>>
where
    T: Send + 'static,
    E: Send + 'static,
    F: Future<Output = Result<T, E>> + Send + 'static,
{
    let owner = OwnerHandle::current().expect("spawn() called outside a reactive scope");
    HOST.with(|slot| {
        let parts = slot
            .borrow()
            .as_ref()
            .expect("spawn() requires AsyncRuntime::install_current")
            .clone();
        spawn_with(parts, owner, future)
    })
}

#[must_use]
#[track_caller]
pub fn signal_writer<T>(signal: Signal<T>) -> SignalWriter<T>
where
    T: Send + 'static,
{
    let owner = OwnerHandle::current().expect("signal_writer() called outside a reactive scope");
    HOST.with(|slot| {
        let parts = slot
            .borrow()
            .as_ref()
            .expect("signal_writer() requires AsyncRuntime::install_current")
            .clone();
        SignalWriter::new(parts.dispatcher, owner, signal)
    })
}
