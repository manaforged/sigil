//! Thread-local spawn host installed by [`AsyncRuntime::install_current`].

use std::cell::RefCell;
use std::future::Future;

use sigil_reactive::Signal;

use crate::dispatch::spawn_with;
use crate::handle::OwnerHandle;
use crate::runtime::SpawnParts;
use crate::status::Status;

thread_local! {
    static HOST: RefCell<Option<SpawnParts>> = const { RefCell::new(None) };
}

pub(crate) fn install(parts: SpawnParts) {
    HOST.with(|slot| *slot.borrow_mut() = Some(parts));
}

/// Spawn `future` on the current runtime and return a
/// [`Signal<Status<T, E>>`].
///
/// Uses the owner of the current reactive scope. `sigil::launch`
/// installs the runtime; tests call
/// [`AsyncRuntime::install_current`](crate::AsyncRuntime::install_current).
///
/// # Panics
///
/// Panics if called outside a reactive scope, or if no runtime is
/// installed on this thread.
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
