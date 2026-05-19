//! `Context::spawn_local` — the `!Send`-future companion to
//! [`crate::cx_spawn::Context::spawn`] (W4.99.B3 amendment to DR-048).
//!
//! The original [`Context::spawn`] forces `F: Future + Send + 'static`
//! because the production executor (Tokio multi-thread) moves the
//! boxed future across worker threads. Authors that need to hold a
//! `!Send` value across `.await` — `Rc<RefCell<_>>`, a non-`Send`
//! third-party handle, an iterator over a non-shared resource — could
//! not satisfy that bound, forcing awkward refactors like wrapping
//! everything in `Arc<Mutex<_>>`.
//!
//! `spawn_local` lifts the `Send` requirement. The boxed future is
//! handed to a [`crate::LocalTaskExecutor`] that drives it on the main
//! thread (a `tokio::task::LocalSet` in production). The result is
//! delivered through a `tokio::sync::oneshot` whose sender lives in
//! the future body and whose receiver lives in the returned
//! [`crate::LocalTask`]; since both halves stay on the main thread,
//! the channel does not need to ferry the value across threads even
//! when `R: !Send`.
//!
//! The signature mirrors [`Context::spawn`] in every respect except
//! the dropped `Send` bounds.

use core::future::Future;
use std::sync::Arc;

use tokio::sync::oneshot;

use crate::async_cx::AsyncContext;
use crate::cx::Context;
use crate::local_executor::{BoxLocalFuture, LocalTaskExecutor};
use crate::owner::OwnerId;
use crate::runtime::{EntityKey, OwnerKey, with_rt};
use crate::task_local::LocalTask;

impl<T: 'static> Context<'_, T> {
    /// Spawn a `!Send` future tied to this entity's scope (W4.99.B3).
    ///
    /// Use this when the future body needs to hold a value that is
    /// not `Send` across an `.await` — for example, an `Rc<RefCell<_>>`
    /// captured from the surrounding scope. The future runs on the
    /// installed [`crate::LocalTaskExecutor`] (typically a
    /// `tokio::task::LocalSet`) on the main thread; the returned
    /// [`LocalTask<R>`] cancels-on-drop unless [`LocalTask::detach`]
    /// is called.
    ///
    /// Owner-cascade disposal of the spawning entity also aborts the
    /// task synchronously through the cleanup registered here, mirroring
    /// the [`Context::spawn`] behaviour.
    ///
    /// Per DR-048 amendment B3, this method is a write seam — same as
    /// [`Context::spawn`] — and panics during
    /// [`crate::DrawPhase::Paint`].
    ///
    /// # Panics
    ///
    /// Panics if called during [`crate::DrawPhase::Paint`]. Also panics
    /// if [`crate::Runtime::install_local_executor`] has not been
    /// called — there is no local executor to spawn against. Tests
    /// must install a fixture before exercising `spawn_local`.
    #[track_caller]
    pub fn spawn_local<F, R>(
        &mut self,
        future_fn: impl FnOnce(AsyncContext<T>) -> F + 'static,
    ) -> LocalTask<R>
    where
        F: Future<Output = R> + 'static,
        R: 'static,
    {
        let entity_key = self.entity;
        let (owner_key, local_executor, dispatcher) = with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "Context::spawn_local");
            let owner = rt
                .entities
                .get(entity_key)
                .expect("Context::spawn_local on a disposed entity")
                .owner;
            let local_executor = rt
                .local_executor
                .as_ref()
                .expect(
                    "Context::spawn_local requires \
                     Runtime::install_local_executor — no local executor \
                     installed on this thread",
                )
                .clone();
            // The dispatcher is still required because `AsyncContext`
            // re-entry to the main thread goes through it. The local
            // executor drives the future itself; the dispatcher carries
            // the closures `acx.update` posts back to the reactive runtime.
            let dispatcher = rt
                .dispatcher
                .as_ref()
                .expect(
                    "Context::spawn_local requires Runtime::install_executor — \
                     no dispatcher installed on this thread",
                )
                .clone();
            (owner, local_executor, dispatcher)
        });
        spawn_local_through(
            entity_key,
            owner_key,
            &local_executor,
            &dispatcher,
            future_fn,
        )
    }
}

/// Hand-off from [`Context::spawn_local`] to the local executor. Lives
/// at module scope so the generic monomorphises in one place; keeps
/// the per-call surface on `Context` small (mirrors `spawn_through`).
fn spawn_local_through<T, F, R>(
    entity_key: EntityKey,
    owner_key: OwnerKey,
    local_executor: &Arc<dyn LocalTaskExecutor>,
    dispatcher: &Arc<dyn crate::async_cx::MainThreadDispatcher>,
    future_fn: impl FnOnce(AsyncContext<T>) -> F + 'static,
) -> LocalTask<R>
where
    T: 'static,
    F: Future<Output = R> + 'static,
    R: 'static,
{
    let owner_id = OwnerId(owner_key);
    let acx = AsyncContext::<T>::new(entity_key, owner_id, Arc::clone(dispatcher));
    let (tx, rx) = oneshot::channel::<R>();
    // Build the boxed `!Send` future: drive the user body and post the
    // result through the oneshot. Ignore send errors — the receiver may
    // have been dropped if the LocalTask handle was dropped first.
    let fut: BoxLocalFuture = Box::pin(async move {
        let user_fut = future_fn(acx);
        let value = user_fut.await;
        let _ = tx.send(value);
    });
    let abort = local_executor.spawn_local(owner_id, fut);
    // Register owner-cascade abort: if the entity's owner disposes,
    // the cleanup runs and aborts the task. Same shape as `spawn`.
    let abort_for_cleanup = abort.clone();
    with_rt(|rt| {
        if let Some(slot) = rt.owners.get_mut(owner_key) {
            slot.cleanups
                .push(Box::new(move || abort_for_cleanup.abort()));
        }
    });
    LocalTask::<R>::from_parts(abort, rx)
}
