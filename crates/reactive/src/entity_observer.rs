//! Observer machinery for entities — `Subscription`, `observe`,
//! `subscribe`, and the end-of-turn drain.
//!
//! See DR-045 §4 (Subscription RAII + owner-cascade safety net) and §5
//! (batched notify drain). All callbacks run *outside* the originating
//! `update` closure (after its runtime borrow has released) which makes
//! "observe-then-mutate-another" structurally safe.

use core::any::Any;
use core::marker::PhantomData;

use crate::cx::Context;
use crate::entity::Entity;
use crate::owner::on_cleanup;
use crate::runtime::{
    EntityKey, ObserveBareFn, ObserveFn, ObserverKey, ObserverKind, ObserverSlot, RuntimeState,
    SubscribeFn, with_rt,
};

/// RAII handle to an active observation. Dropping it unsubscribes.
///
/// Marked `#[must_use]` — `cx.observe(&e, |...|)` without binding the
/// return is a usage bug. Binding to `_sub` or `let _ = ...` is
/// supported: the owner-cleanup safety net also unsubscribes when the
/// enclosing reactive owner disposes (see DR-045 §4).
#[must_use = "subscriptions unsubscribe on drop; bind the handle to keep observation alive"]
pub struct Subscription {
    pub(crate) key: ObserverKey,
    // !Send + !Sync — observer lists are thread-local.
    _not_send: PhantomData<*const ()>,
}

impl core::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Subscription({:?})", self.key)
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        unsubscribe(self.key);
    }
}

/// Remove an observer from the runtime — idempotent, called from both
/// `Subscription::drop` and the owner-cascade cleanup.
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
        // Drop any still-pending queue entries that reference this
        // observer. Cheap linear scan; the queue is bounded by the
        // number of writes per turn.
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

/// Register an owner-cascade `on_cleanup` that removes `key` if it is
/// still installed when the owner disposes. Idempotent with the
/// explicit `Subscription::drop` path.
fn register_cleanup(key: ObserverKey) {
    on_cleanup(move || {
        unsubscribe(key);
    });
}

/// Register a plain observer — fires `callback` on any `cx.notify()`
/// against `observed`. Called from `Context::observe`.
#[track_caller]
pub(crate) fn register_observe<T, U>(
    observer_entity: EntityKey,
    observed: Entity<U>,
    mut callback: impl FnMut(&mut T, &Entity<U>, &mut Context<'_, T>) + 'static,
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
    Subscription {
        key,
        _not_send: PhantomData,
    }
}

/// Register a typed-event subscriber — fires only on `cx.emit::<Ev>`.
#[track_caller]
pub(crate) fn register_subscribe<T, U, Ev>(
    observer_entity: EntityKey,
    observed: Entity<U>,
    mut callback: impl FnMut(&mut T, &Entity<U>, &Ev, &mut Context<'_, T>) + 'static,
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
    Subscription {
        key,
        _not_send: PhantomData,
    }
}

/// Register a top-level `observe_global` — bare `FnMut()` with no
/// observer entity. Used by [`crate::entity_global::observe_global`].
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
    Subscription {
        key,
        _not_send: PhantomData,
    }
}

fn current_owner() -> crate::runtime::OwnerKey {
    with_rt(|rt| {
        rt.current_owner
            .expect("observer registration outside a reactive scope")
    })
}

type ObserveCallback<T, U> = dyn FnMut(&mut T, &Entity<U>, &mut Context<'_, T>) + 'static;
type SubscribeCallback<T, U, Ev> =
    dyn FnMut(&mut T, &Entity<U>, &Ev, &mut Context<'_, T>) + 'static;

/// Dispatch a plain-observe callback. Pulls the observer's payload out
/// of its slot for the duration of the call, restoring on return.
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
            // Type mismatch — restore and abandon.
            with_rt(|rt| {
                if let Some(slot) = rt.entities.get_mut(entity_key) {
                    slot.value = value;
                }
            });
            return;
        };
        let mut cx = Context::<'_, T>::new_unchecked(entity_key);
        callback(inner, &observed, &mut cx);
    }
    with_rt(|rt| {
        if let Some(slot) = rt.entities.get_mut(entity_key) {
            slot.value = value;
        }
    });
}

/// Dispatch a `subscribe::<U, Ev>` callback with the typed event.
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
        let mut cx = Context::<'_, T>::new_unchecked(entity_key);
        callback(inner, &observed, ev, &mut cx);
    }
    with_rt(|rt| {
        if let Some(slot) = rt.entities.get_mut(entity_key) {
            slot.value = value;
        }
    });
}

/// Drain the pending-observers queue. Called from `Entity::new` (at
/// top-level scope) and from `entity::run_update` when the outermost
/// `update` exits (depth returns to 0).
///
/// Re-entrant: a callback can itself call `Entity::update` which
/// enqueues more observers; those are picked up by this same drain
/// loop since we keep popping until the queue is empty.
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
    // Take the observer kind out of the slot so we can mutate the
    // closure without holding the runtime borrow. A sentinel
    // (ObserveBare with a no-op closure) sits in the slot until we
    // restore the real kind.
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
        // If the observer was removed mid-callback (e.g. its
        // Subscription dropped during dispatch), the kind drops here.
    });
}
