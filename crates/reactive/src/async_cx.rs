use core::any::Any;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio::task::AbortHandle;

use crate::entity::Entity;
use crate::owner::OwnerId;
use crate::runtime::{EntityKey, with_rt};
use crate::task::EntityGone;

pub type BoxSendFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub type BoxMainCallback = Box<dyn FnOnce() + Send + 'static>;

pub trait TaskExecutor: Send + Sync + 'static {
    fn spawn(&self, owner: OwnerId, fut: BoxSendFuture) -> AbortHandle;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    Closed,
}

impl core::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Closed => f.write_str("main-thread dispatcher is closed"),
        }
    }
}

impl std::error::Error for DispatchError {}

pub trait MainThreadDispatcher: Send + Sync + 'static {
    fn post(&self, owner: OwnerId, callback: BoxMainCallback) -> Result<(), DispatchError>;
}

pub struct AsyncContext<T: 'static> {
    pub(crate) entity_key: EntityKey,
    pub(crate) owner: OwnerId,
    pub(crate) dispatcher: Arc<dyn MainThreadDispatcher>,
    _t: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for AsyncContext<T> {
    fn clone(&self) -> Self {
        Self {
            entity_key: self.entity_key,
            owner: self.owner,
            dispatcher: Arc::clone(&self.dispatcher),
            _t: PhantomData,
        }
    }
}

impl<T: 'static> core::fmt::Debug for AsyncContext<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AsyncContext")
            .field("entity", &self.entity_key)
            .field("owner", &self.owner)
            .finish_non_exhaustive()
    }
}

impl<T: 'static> AsyncContext<T> {
    #[allow(
        dead_code,
        reason = "consumed by EntityCx::spawn in the same DR-048 landing — module landing order"
    )]
    pub(crate) fn new(
        entity_key: EntityKey,
        owner: OwnerId,
        dispatcher: Arc<dyn MainThreadDispatcher>,
    ) -> Self {
        Self {
            entity_key,
            owner,
            dispatcher,
            _t: PhantomData,
        }
    }

    pub async fn update<R, F>(&self, f: F) -> Result<R, EntityGone>
    where
        F: FnOnce(&mut T, &mut crate::EntityCx<'_, T>) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel::<Result<R, EntityGone>>();
        let entity_key = self.entity_key;
        let callback: BoxMainCallback = Box::new(move || {
            let outcome = run_update_on_main::<T, R>(entity_key, f);
            let _ = tx.send(outcome);
        });
        if self.dispatcher.post(self.owner, callback).is_err() {
            return Err(EntityGone);
        }
        rx.await.unwrap_or(Err(EntityGone))
    }

    pub fn update_peer<V, R, F>(
        &self,
        entity: &Entity<V>,
        f: F,
    ) -> impl core::future::Future<Output = Result<R, EntityGone>> + Send + 'static
    where
        V: 'static,
        F: FnOnce(&mut V, &mut crate::EntityCx<'_, V>) -> R + Send + 'static,
        R: Send + 'static,
    {
        let peer_key = entity.key;
        let owner = self.owner;
        let dispatcher = Arc::clone(&self.dispatcher);
        async move {
            let (tx, rx) = oneshot::channel::<Result<R, EntityGone>>();
            let callback: BoxMainCallback = Box::new(move || {
                let outcome = run_update_on_main::<V, R>(peer_key, f);
                let _ = tx.send(outcome);
            });
            if dispatcher.post(owner, callback).is_err() {
                return Err(EntityGone);
            }
            rx.await.unwrap_or(Err(EntityGone))
        }
    }

    pub async fn read<R, F>(&self, f: F) -> Result<R, EntityGone>
    where
        F: FnOnce(&T) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel::<Result<R, EntityGone>>();
        let entity_key = self.entity_key;
        let callback: BoxMainCallback = Box::new(move || {
            let outcome = run_read_on_main::<T, R>(entity_key, f);
            let _ = tx.send(outcome);
        });
        if self.dispatcher.post(self.owner, callback).is_err() {
            return Err(EntityGone);
        }
        rx.await.unwrap_or(Err(EntityGone))
    }
}

fn run_update_on_main<U: 'static, R>(
    key: EntityKey,
    f: impl FnOnce(&mut U, &mut crate::EntityCx<'_, U>) -> R + Send + 'static,
) -> Result<R, EntityGone> {
    let alive = with_rt(|rt| rt.entities.contains(key));
    if !alive {
        return Err(EntityGone);
    }
    let result = crate::entity::run_update::<U, R>(key, core::panic::Location::caller(), f);
    Ok(result)
}

fn run_read_on_main<U: 'static, R>(
    key: EntityKey,
    f: impl FnOnce(&U) -> R + Send + 'static,
) -> Result<R, EntityGone> {
    let value: Box<dyn Any> = match with_rt(|rt| {
        rt.entities
            .get_mut(key)
            .map(|slot| core::mem::replace(&mut slot.value, Box::new(())) as Box<dyn Any>)
    }) {
        Some(v) => v,
        None => return Err(EntityGone),
    };
    let mut guard = ReadRestoreGuard {
        key,
        value: Some(value),
    };
    let result = {
        let inner = guard
            .value
            .as_ref()
            .expect("populated by guard ctor")
            .downcast_ref::<U>()
            .expect("entity type mismatch in AsyncContext::read");
        f(inner)
    };
    let restored = guard.value.take().expect("value present");
    with_rt(|rt| {
        if let Some(slot) = rt.entities.get_mut(key) {
            slot.value = restored;
        }
    });
    Ok(result)
}

struct ReadRestoreGuard {
    key: EntityKey,
    value: Option<Box<dyn Any>>,
}

impl Drop for ReadRestoreGuard {
    fn drop(&mut self) {
        if let Some(v) = self.value.take() {
            with_rt(|rt| {
                if let Some(slot) = rt.entities.get_mut(self.key) {
                    slot.value = v;
                }
            });
        }
    }
}

const _: fn() = || {
    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}
    const fn assert_clone<T: Clone>() {}
    const fn assert_static<T: 'static>() {}
    assert_send::<AsyncContext<()>>();
    assert_sync::<AsyncContext<()>>();
    assert_clone::<AsyncContext<()>>();
    assert_static::<AsyncContext<()>>();
};
