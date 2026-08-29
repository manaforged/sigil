use core::future::Future;
use std::sync::Arc;

use tokio::sync::oneshot;

use crate::async_cx::AsyncContext;
use crate::cx::EntityCx;
use crate::local_executor::{BoxLocalFuture, LocalTaskExecutor};
use crate::owner::OwnerId;
use crate::runtime::{EntityKey, OwnerKey, with_rt};
use crate::task_local::LocalTask;

impl<T: 'static> EntityCx<'_, T> {
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
            crate::phase::assert_paint_safe(rt, "EntityCx::spawn_local");
            let owner = rt
                .entities
                .get(entity_key)
                .expect("EntityCx::spawn_local on a disposed entity")
                .owner;
            let local_executor = rt
                .local_executor
                .as_ref()
                .expect(
                    "EntityCx::spawn_local requires \
                     Runtime::install_local_executor — no local executor \
                     installed on this thread",
                )
                .clone();
            let dispatcher = rt
                .dispatcher
                .as_ref()
                .expect(
                    "EntityCx::spawn_local requires Runtime::install_executor — \
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
    let fut: BoxLocalFuture = Box::pin(async move {
        let user_fut = future_fn(acx);
        let value = user_fut.await;
        let _ = tx.send(value);
    });
    let abort = local_executor.spawn_local(owner_id, fut);
    let abort_for_cleanup = abort.clone();
    with_rt(|rt| {
        if let Some(slot) = rt.owners.get_mut(owner_key) {
            slot.cleanups
                .push(Box::new(move || abort_for_cleanup.abort()));
        }
    });
    LocalTask::<R>::from_parts(abort, rx)
}
