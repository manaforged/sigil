use std::future::Future;

use sigil_reactive::{Signal, signal};

use crate::handle::OwnerHandle;
use crate::runtime::{AsyncRuntime, DispatchMessage, Dispatcher, register_abort};
use crate::status::Status;

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
    parts: crate::runtime::SpawnParts,
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
    let _ = sigil_reactive::owner_by_id(owner.id(), move || {
        sigil_reactive::on_cleanup(move || {
            abort.abort();
        });
    });
    sig
}
