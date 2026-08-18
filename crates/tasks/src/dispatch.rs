//! Public async entry points.
//!
//! [`dispatch_to_owner`] is the cross-thread main-thread trampoline.
//! [`spawn_to_signal`] is the reactive wrapper built on top of it
//! that turns a background future into a `Signal<Status<T, E>>`.
//!
//! Both take a [`Dispatcher`] (or an [`AsyncRuntime`] for the
//! main-thread convenience wrapper) and an [`OwnerHandle`], and
//! inherit the handle's disposal-cancellation semantics: if the
//! owner has been disposed by the time the scheduler drains, the
//! callback is dropped silently (see DR-005 addendum §3 and DR-007
//! invariant (4)).

use std::future::Future;

use sigil_reactive::{Signal, signal};

use crate::handle::OwnerHandle;
use crate::runtime::{AsyncRuntime, DispatchMessage, Dispatcher, register_abort};
use crate::status::Status;

/// Post a closure to run on the main thread with `handle`'s owner
/// active.
///
/// The closure is enqueued via the dispatcher's MPSC and runs on
/// the next [drain][sigil_hal::Scheduler::drain]. If `handle`'s owner
/// has been disposed by then, the closure is dropped silently —
/// no panic, no error. This is the "no-op target" contract from
/// DR-005 addendum §3.
///
/// The closure runs with the owner set as the current reactive
/// owner, so it may freely call `on_cleanup`, write signals owned
/// by that scope, or create nested reactive primitives parented
/// against it. It must remain `Send + 'static` because it crosses
/// the MPSC.
///
/// # Arguments
///
/// * `dispatcher` — a `Send + Sync` handle obtained from
///   [`AsyncRuntime::dispatcher`]. Background tasks move a clone
///   of the dispatcher into their `async move` body; main-thread
///   code can call `rt.dispatcher()` inline.
/// * `handle` — the scope the closure should run against.
/// * `f` — the closure to run.
///
/// If the dispatcher channel is closed (runtime torn down during
/// shutdown), the closure is silently dropped — same behaviour as
/// [`spawn_to_signal`]'s internal dispatch path.
pub fn dispatch_to_owner<F>(dispatcher: &Dispatcher, handle: OwnerHandle, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let _ = dispatcher.send(DispatchMessage {
        owner: handle,
        callback: Box::new(f),
    });
}

/// Spawn `future` on the runtime and drive a reactive
/// `Signal<Status<T, E>>` from its completion.
///
/// The signal starts as [`Status::Pending`]. When the future
/// completes the dispatcher writes [`Status::Ok`] or
/// [`Status::Err`] into the signal, re-running any effect that read
/// it. If the owning scope disposes before the future resolves, the
/// task is aborted **synchronously** (via an `on_cleanup` hook
/// registered at spawn time) and no further writes happen.
///
/// Must be called from the thread that owns the reactive runtime
/// (the main thread). Internally it creates the signal synchronously,
/// spawns the future, and registers an `on_cleanup` against `owner`
/// so that disposing the owner aborts the task the instant disposal
/// runs — not at the next drain.
///
/// # Stale-owner behaviour
///
/// If `owner` is already disposed at the call site, the returned
/// signal is still created (parented to the current reactive owner),
/// the future is dropped unpolled, and the signal stays at
/// `Pending` forever. Callers that want to observe this should
/// check `owner.is_alive()` before calling.
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

/// Drive a background future onto a [`Signal`] using captured runtime parts.
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
