use core::any::Any;
use core::marker::PhantomData;

use crate::cx::EntityCx;
use crate::entity::Entity;
use crate::owner::on_cleanup;
use crate::runtime::{
    EntityKey, ObserveBareFn, ObserveFn, ObserverKey, ObserverKind, ObserverSlot, RuntimeState,
    SubscribeFn, with_rt,
};

#[must_use = "subscriptions unsubscribe on drop; bind the handle to keep observation alive"]
pub struct Subscription {
    pub(crate) kind: SubscriptionKind,
    _not_send: PhantomData<*const ()>,
}

pub(crate) enum SubscriptionKind {
    Observer(ObserverKey),
    Cleanup(Option<Box<dyn FnOnce()>>),
}

impl core::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.kind {
            SubscriptionKind::Observer(key) => write!(f, "Subscription({key:?})"),
            SubscriptionKind::Cleanup(_) => write!(f, "Subscription(<cleanup>)"),
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        match &mut self.kind {
            SubscriptionKind::Observer(key) => unsubscribe(*key),
            SubscriptionKind::Cleanup(cleanup) => {
                if let Some(cleanup) = cleanup.take() {
                    cleanup();
                }
            }
        }
    }
}

impl Subscription {
    pub(crate) const fn observer(key: ObserverKey) -> Self {
        Self {
            kind: SubscriptionKind::Observer(key),
            _not_send: PhantomData,
        }
    }

    pub(crate) fn cleanup(cleanup: impl FnOnce() + 'static) -> Self {
        Self {
            kind: SubscriptionKind::Cleanup(Some(Box::new(cleanup))),
            _not_send: PhantomData,
        }
    }
}

pub(crate) fn unsubscribe(key: ObserverKey) {
    with_rt(|rt| {
        let Some(slot) = rt.observers.remove(key) else {
            return;
        };
        if let Some(observed) = rt.entities.get_mut(slot.observed) {
            if let Some(pos) = observed.subscribers.iter().position(|k| *k == key) {
                observed.subscribers.swap_remove(pos);
            }
            if let Some(pos) = observed.event_subs.iter().position(|(_, k)| *k == key) {
                observed.event_subs.swap_remove(pos);
            }
        }
        rt.pending_observers.retain(|p| p.observer != key);
    });
}

fn install_observer(
    slot: ObserverSlot,
    observed: EntityKey,
    event_ty: Option<core::any::TypeId>,
) -> ObserverKey {
    with_rt(|rt| {
        let owner = slot.owner;
        let key = rt.observers.insert(slot);
        if let Some(entity) = rt.entities.get_mut(observed) {
            if let Some(ty) = event_ty {
                entity.event_subs.push((ty, key));
            } else {
                entity.subscribers.push(key);
            }
        }
        if let Some(owner_slot) = rt.owners.get_mut(owner) {
            owner_slot.observers.push(key);
        }
        key
    })
}

fn register_cleanup(key: ObserverKey) {
    on_cleanup(move || {
        unsubscribe(key);
    });
}

#[track_caller]
pub(crate) fn register_observe<T, U>(
    observer_entity: EntityKey,
    observed: Entity<U>,
    mut callback: impl FnMut(&mut T, &Entity<U>, &mut EntityCx<'_, T>) + 'static,
) -> Subscription
where
    T: 'static,
    U: 'static,
{
    let owner = current_owner();
    let observe_fn: ObserveFn = Box::new(move |observer_key| {
        dispatch_observe::<T, U>(observer_key, observed, &mut callback);
    });
    let slot = ObserverSlot {
        observer_entity: Some(observer_entity),
        observed: observed.key,
        owner,
        kind: ObserverKind::Observe(observe_fn),
    };
    let key = install_observer(slot, observed.key, None);
    register_cleanup(key);
    Subscription::observer(key)
}

#[track_caller]
pub(crate) fn register_subscribe<T, U, Ev>(
    observer_entity: EntityKey,
    observed: Entity<U>,
    mut callback: impl FnMut(&mut T, &Entity<U>, &Ev, &mut EntityCx<'_, T>) + 'static,
) -> Subscription
where
    T: 'static,
    U: 'static,
    Ev: 'static,
{
    let owner = current_owner();
    let ev_type = core::any::TypeId::of::<Ev>();
    let sub_fn: SubscribeFn = Box::new(move |observer_key, payload| {
        if let Some(ev) = payload.downcast_ref::<Ev>() {
            dispatch_subscribe::<T, U, Ev>(observer_key, observed, ev, &mut callback);
        }
    });
    let slot = ObserverSlot {
        observer_entity: Some(observer_entity),
        observed: observed.key,
        owner,
        kind: ObserverKind::Subscribe { ev_type, f: sub_fn },
    };
    let key = install_observer(slot, observed.key, Some(ev_type));
    register_cleanup(key);
    Subscription::observer(key)
}

#[track_caller]
pub(crate) fn register_observe_global(
    observed: EntityKey,
    callback: impl FnMut() + 'static,
) -> Subscription {
    let owner = current_owner();
    let bare: ObserveBareFn = Box::new(callback);
    let slot = ObserverSlot {
        observer_entity: None,
        observed,
        owner,
        kind: ObserverKind::ObserveBare(bare),
    };
    let key = install_observer(slot, observed, None);
    register_cleanup(key);
    Subscription::observer(key)
}

fn current_owner() -> crate::runtime::OwnerKey {
    with_rt(|rt| {
        rt.current_owner
            .expect("observer registration outside a reactive scope")
    })
}

type ObserveCallback<T, U> = dyn FnMut(&mut T, &Entity<U>, &mut EntityCx<'_, T>) + 'static;
type SubscribeCallback<T, U, Ev> =
    dyn FnMut(&mut T, &Entity<U>, &Ev, &mut EntityCx<'_, T>) + 'static;

fn dispatch_observe<T, U>(
    observer_key: ObserverKey,
    observed: Entity<U>,
    callback: &mut ObserveCallback<T, U>,
) where
    T: 'static,
    U: 'static,
{
    let observer_entity = with_rt(|rt| {
        rt.observers
            .get(observer_key)
            .and_then(|s| s.observer_entity)
    });
    let Some(entity_key) = observer_entity else {
        return;
    };
    let taken = with_rt(|rt| {
        rt.entities
            .get_mut(entity_key)
            .map(|slot| core::mem::replace(&mut slot.value, Box::new(())) as Box<dyn Any>)
    });
    let Some(mut value) = taken else { return };
    {
        let Some(inner) = value.downcast_mut::<T>() else {
            with_rt(|rt| {
                if let Some(slot) = rt.entities.get_mut(entity_key) {
                    slot.value = value;
                }
            });
            return;
        };
        let mut cx = EntityCx::<'_, T>::new_unchecked(entity_key);
        callback(inner, &observed, &mut cx);
    }
    with_rt(|rt| {
        if let Some(slot) = rt.entities.get_mut(entity_key) {
            slot.value = value;
        }
    });
}

fn dispatch_subscribe<T, U, Ev>(
    observer_key: ObserverKey,
    observed: Entity<U>,
    ev: &Ev,
    callback: &mut SubscribeCallback<T, U, Ev>,
) where
    T: 'static,
    U: 'static,
    Ev: 'static,
{
    let observer_entity = with_rt(|rt| {
        rt.observers
            .get(observer_key)
            .and_then(|s| s.observer_entity)
    });
    let Some(entity_key) = observer_entity else {
        return;
    };
    let taken = with_rt(|rt| {
        rt.entities
            .get_mut(entity_key)
            .map(|slot| core::mem::replace(&mut slot.value, Box::new(())) as Box<dyn Any>)
    });
    let Some(mut value) = taken else { return };
    {
        let Some(inner) = value.downcast_mut::<T>() else {
            with_rt(|rt| {
                if let Some(slot) = rt.entities.get_mut(entity_key) {
                    slot.value = value;
                }
            });
            return;
        };
        let mut cx = EntityCx::<'_, T>::new_unchecked(entity_key);
        callback(inner, &observed, ev, &mut cx);
    }
    with_rt(|rt| {
        if let Some(slot) = rt.entities.get_mut(entity_key) {
            slot.value = value;
        }
    });
}

pub(crate) fn drain_pending_observers() {
    loop {
        let next = with_rt(|rt| rt.pending_observers.pop_front());
        let Some(pending) = next else {
            return;
        };
        dispatch_one(pending.observer, pending.event);
    }
}

fn dispatch_one(observer: ObserverKey, event: Option<Box<dyn Any>>) {
    enum Taken {
        Observe(ObserveFn),
        Bare(ObserveBareFn),
        Sub {
            ev_type: core::any::TypeId,
            f: SubscribeFn,
        },
        Gone,
    }
    let taken = with_rt(|rt: &mut RuntimeState| {
        let Some(slot) = rt.observers.get_mut(observer) else {
            return Taken::Gone;
        };
        let sentinel = ObserverKind::ObserveBare(Box::new(|| {}));
        let kind = core::mem::replace(&mut slot.kind, sentinel);
        match kind {
            ObserverKind::Observe(f) => Taken::Observe(f),
            ObserverKind::ObserveBare(f) => Taken::Bare(f),
            ObserverKind::Subscribe { ev_type, f } => Taken::Sub { ev_type, f },
        }
    });
    match taken {
        Taken::Gone => {}
        Taken::Observe(mut f) => {
            f(observer);
            restore_kind(observer, ObserverKind::Observe(f));
        }
        Taken::Bare(mut f) => {
            f();
            restore_kind(observer, ObserverKind::ObserveBare(f));
        }
        Taken::Sub { ev_type, mut f } => {
            if let Some(ev) = event {
                f(observer, &*ev);
            }
            restore_kind(observer, ObserverKind::Subscribe { ev_type, f });
        }
    }
}

fn restore_kind(observer: ObserverKey, kind: ObserverKind) {
    with_rt(|rt| {
        if let Some(slot) = rt.observers.get_mut(observer) {
            slot.kind = kind;
        }
    });
}
