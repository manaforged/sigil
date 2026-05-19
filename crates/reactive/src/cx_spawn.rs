//! `Context::spawn` and its `spawn_through` helper (DR-048 §2).
//!
//! Extracted from `cx.rs` to keep that file under the 400-line cap.
//! The generic `spawn_through` lives at module scope so the
//! monomorphisation happens in one place, keeping the per-call
//! surface on [`Context`] small.

use core::future::Future;
use std::sync::Arc;

use tokio::sync::oneshot;

use crate::async_cx::{AsyncContext, BoxSendFuture, MainThreadDispatcher, TaskExecutor};
use crate::cx::Context;
use crate::owner::OwnerId;
use crate::runtime::{EntityKey, OwnerKey, with_rt};
use crate::task::Task;

impl<T: 'static> Context<'_, T> {
    /// Spawn a `Send + 'static` future tied to this entity's scope
    /// (DR-048 §2).
    ///
    /// The returned [`Task<R>`] cancels-on-drop unless [`Task::detach`]
    /// is called. Owner-cascade disposal of the spawning entity also
    /// aborts the task synchronously through the cleanup registered
    /// here.
    ///
    /// Per the DR-048 amendment (B3, 2026-05-18), `Context::spawn`
    /// panics during [`crate::DrawPhase::Paint`] just like every other
    /// reactive write seam. The original "Paint-legal" carve-out was
    /// removed: routing a spawn through Paint hides side-effects that
    /// authors expect to be Paint-illegal, and the completion-runs-in-
    /// Effect argument does not extend to the spawn itself, only to
    /// the completion callback. Handlers that need to spawn during a
    /// paint should either defer through `dispatch_action` or restructure
    /// the call site to happen in Effect.
    ///
    /// # Panics
    ///
    /// Panics if called during [`crate::DrawPhase::Paint`] (DR-048 §B3
    /// amendment) — matches every other reactive write seam. Also
    /// panics if [`crate::Runtime::install_executor`] has not been
    /// called — there is no executor to spawn against. Tests must
    /// install a fixture pair before exercising `spawn`.
    #[track_caller]
    pub fn spawn<F, R>(
        &mut self,
        future_fn: impl FnOnce(AsyncContext<T>) -> F + Send + 'static,
    ) -> Task<R>
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        let entity_key = self.entity;
        // Resolve the entity's owner BEFORE crossing the user closure.
        let (owner_key, executor, dispatcher) = with_rt(|rt| {
            // DR-048 amendment B3 — spawn is a write seam; Paint-phase
            // spawn panics in debug (matches Signal::set, Entity::update,
            // Context::notify, Context::emit, Context::focus, set_global).
            crate::phase::assert_paint_safe(rt, "Context::spawn");
            let owner = rt
                .entities
                .get(entity_key)
                .expect("Context::spawn on a disposed entity")
                .owner;
            let executor = rt
                .executor
                .as_ref()
                .expect(
                    "Context::spawn requires Runtime::install_executor — \
                     no executor installed on this thread",
                )
                .clone();
            let dispatcher = rt
                .dispatcher
                .as_ref()
                .expect(
                    "Context::spawn requires Runtime::install_executor — \
                     no dispatcher installed on this thread",
                )
                .clone();
            (owner, executor, dispatcher)
        });
        spawn_through(entity_key, owner_key, &executor, &dispatcher, future_fn)
    }
}

/// Hand-off from [`Context::spawn`] to the executor. Lives at module
/// scope so the generic monomorphises in one place; keeps the
/// per-call surface in `spawn` small.
fn spawn_through<T, F, R>(
    entity_key: EntityKey,
    owner_key: OwnerKey,
    executor: &Arc<dyn TaskExecutor>,
    dispatcher: &Arc<dyn MainThreadDispatcher>,
    future_fn: impl FnOnce(AsyncContext<T>) -> F + Send + 'static,
) -> Task<R>
where
    T: 'static,
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let owner_id = OwnerId(owner_key);
    let acx = AsyncContext::<T>::new(entity_key, owner_id, Arc::clone(dispatcher));
    let (tx, rx) = oneshot::channel::<R>();
    // Build the boxed future: drive the user body and post the result
    // through the oneshot. Ignore send errors — the receiver may have
    // been dropped if the Task<R> handle was dropped first.
    let fut: BoxSendFuture = Box::pin(async move {
        let user_fut = future_fn(acx);
        let value = user_fut.await;
        let _ = tx.send(value);
    });
    let abort = executor.spawn(owner_id, fut);
    // Register owner-cascade abort: if the entity's owner disposes,
    // the cleanup runs and aborts the task. Idempotent with explicit
    // Drop-abort because `AbortHandle::abort` is a no-op after the
    // task finishes or is already aborted.
    let abort_for_cleanup = abort.clone();
    with_rt(|rt| {
        if let Some(slot) = rt.owners.get_mut(owner_key) {
            slot.cleanups
                .push(Box::new(move || abort_for_cleanup.abort()));
        }
    });
    Task::<R>::from_parts(abort, rx)
}
