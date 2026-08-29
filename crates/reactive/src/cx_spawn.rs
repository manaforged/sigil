use core::future::Future;
use std::sync::Arc;

use tokio::sync::oneshot;

use crate::async_cx::{AsyncContext, BoxSendFuture, MainThreadDispatcher, TaskExecutor};
use crate::cx::EntityCx;
use crate::owner::OwnerId;
use crate::runtime::{EntityKey, OwnerKey, with_rt};
use crate::task::Task;

impl<T: 'static> EntityCx<'_, T> {
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
        let (owner_key, executor, dispatcher) = with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "EntityCx::spawn");
            let owner = rt
                .entities
                .get(entity_key)
                .expect("EntityCx::spawn on a disposed entity")
                .owner;
            let executor = rt
                .executor
                .as_ref()
                .expect(
                    "EntityCx::spawn requires Runtime::install_executor — \
                     no executor installed on this thread",
                )
                .clone();
            let dispatcher = rt
                .dispatcher
                .as_ref()
                .expect(
                    "EntityCx::spawn requires Runtime::install_executor — \
                     no dispatcher installed on this thread",
                )
                .clone();
            (owner, executor, dispatcher)
        });
        spawn_through(entity_key, owner_key, &executor, &dispatcher, future_fn)
    }
}

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
    let fut: BoxSendFuture = Box::pin(async move {
        let user_fut = future_fn(acx);
        let value = user_fut.await;
        let _ = tx.send(value);
    });
    let abort = executor.spawn(owner_id, fut);
    let abort_for_cleanup = abort.clone();
    with_rt(|rt| {
        if let Some(slot) = rt.owners.get_mut(owner_key) {
            slot.cleanups
                .push(Box::new(move || abort_for_cleanup.abort()));
        }
    });
    Task::<R>::from_parts(abort, rx)
}
