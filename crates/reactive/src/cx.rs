use core::any::{Any, TypeId};
use core::marker::PhantomData;
use core::panic::Location;

use smallvec::SmallVec;

use crate::entity::Entity;
use crate::entity_observer::{Subscription, register_observe, register_subscribe};
use crate::owner::on_cleanup;
use crate::runtime::{EntityKey, ObserverKey, PendingObserver, with_rt};

pub struct EntityCx<'a, T: 'static> {
    pub(crate) entity: EntityKey,
    _life: PhantomData<&'a mut T>,
    _not_send: PhantomData<*const ()>,
}

impl<T: 'static> EntityCx<'_, T> {
    pub(crate) const fn new_unchecked(entity: EntityKey) -> Self {
        Self {
            entity,
            _life: PhantomData,
            _not_send: PhantomData,
        }
    }

    pub fn notify(&mut self) {
        let key = self.entity;
        with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "EntityCx::notify");
            let Some(slot) = rt.entities.get(key) else {
                return;
            };
            let subs: SmallVec<[ObserverKey; 4]> = slot.subscribers.clone();
            if !subs.is_empty() {
                rt.frame_dirty = true;
            }
            for observer in subs {
                rt.pending_observers.push_back(PendingObserver {
                    observer,
                    event: None,
                });
            }
        });
    }

    #[track_caller]
    pub fn observe<U: 'static>(
        &mut self,
        observed: &Entity<U>,
        callback: impl FnMut(&mut T, &Entity<U>, &mut EntityCx<'_, T>) + 'static,
    ) -> Subscription {
        register_observe::<T, U>(self.entity, *observed, callback)
    }

    #[track_caller]
    pub fn subscribe<U: 'static, Ev: 'static>(
        &mut self,
        observed: &Entity<U>,
        callback: impl FnMut(&mut T, &Entity<U>, &Ev, &mut EntityCx<'_, T>) + 'static,
    ) -> Subscription {
        register_subscribe::<T, U, Ev>(self.entity, *observed, callback)
    }

    pub fn emit<Ev: Clone + 'static>(&mut self, ev: Ev) {
        let key = self.entity;
        let ev_type = TypeId::of::<Ev>();
        let mut observers: SmallVec<[ObserverKey; 4]> = with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "EntityCx::emit");
            rt.entities.get(key).map_or_else(SmallVec::new, |slot| {
                slot.event_subs
                    .iter()
                    .filter_map(|(ty, ob)| (*ty == ev_type).then_some(*ob))
                    .collect()
            })
        });
        let Some(last) = observers.pop() else {
            return;
        };
        with_rt(|rt| {
            for observer in observers {
                rt.pending_observers.push_back(PendingObserver {
                    observer,
                    event: Some(Box::new(ev.clone())),
                });
            }
            rt.pending_observers.push_back(PendingObserver {
                observer: last,
                event: Some(Box::new(ev)),
            });
        });
    }

    #[track_caller]
    pub fn observe_global<G: 'static>(
        &mut self,
        mut callback: impl FnMut(&mut T, &mut EntityCx<'_, T>) + 'static,
    ) -> Subscription {
        let entity = crate::entity_global::ensure_global_entity::<G>();
        register_observe::<T, G>(self.entity, entity, move |this, _g, cx| {
            callback(this, cx);
        })
    }

    #[track_caller]
    pub fn mutate<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        let caller = Location::caller();
        let key = self.entity;
        let value = with_rt(|rt| {
            let Some(slot) = rt.entities.get_mut(key) else {
                crate::diagnostic::disposed_entity("mutate", caller);
            };
            core::mem::replace(&mut slot.value, Box::new(())) as Box<dyn Any>
        });
        let mut guard = EntityMutGuard {
            key,
            value: Some(value),
        };
        let inner = guard
            .value
            .as_mut()
            .expect("populated by guard ctor")
            .downcast_mut::<T>()
            .expect("entity type mismatch");
        let result = f(inner);
        let restored = guard.value.take().expect("value present");
        with_rt(|rt| {
            if let Some(slot) = rt.entities.get_mut(key) {
                slot.value = restored;
            }
        });
        result
    }

    pub fn on_cleanup(&mut self, f: impl FnOnce() + 'static) {
        on_cleanup(f);
    }
}

struct EntityMutGuard {
    key: EntityKey,
    value: Option<Box<dyn Any>>,
}

impl Drop for EntityMutGuard {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            with_rt(|rt| {
                if let Some(slot) = rt.entities.get_mut(self.key) {
                    slot.value = value;
                }
            });
        }
    }
}
