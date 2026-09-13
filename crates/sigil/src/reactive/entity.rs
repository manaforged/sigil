use core::marker::PhantomData;
use core::panic::Location;

use smallvec::SmallVec;

use crate::reactive::cx::EntityCx;
use crate::reactive::entity_observer::drain_pending_observers;
use crate::reactive::runtime::{EntityKey, EntitySlot, with_rt};

pub struct Entity<T: 'static> {
    pub(crate) key: EntityKey,
    _not_send: PhantomData<*const T>,
}

impl<T: 'static> Clone for Entity<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Entity<T> {}

impl<T: 'static> core::fmt::Debug for Entity<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (name, key_dbg) = with_rt(|rt| {
            rt.entities.get(self.key).map_or_else(
                || ("<disposed>", format!("{:?}", self.key)),
                |slot| (slot.name, format!("{:?}", self.key)),
            )
        });
        write!(
            f,
            "Entity<{}>({key_dbg} {name:?})",
            core::any::type_name::<T>(),
        )
    }
}

impl<T: 'static> Entity<T> {
    pub(crate) const fn from_key(key: EntityKey) -> Self {
        Self {
            key,
            _not_send: PhantomData,
        }
    }

    #[must_use]
    #[track_caller]
    pub fn new(name: &'static str, build: impl FnOnce(&mut EntityCx<'_, T>) -> T) -> Self {
        let location = Location::caller();
        let key = with_rt(|rt| {
            let owner = rt.current_owner.unwrap_or_else(|| {
                crate::reactive::diagnostic::no_scope("Entity::new()", location)
            });
            assert!(
                rt.owners.contains(owner),
                "Entity::new() with a disposed current owner (at {location})"
            );
            let key = rt.entities.insert(EntitySlot {
                owner,
                value: Box::new(()),
                subscribers: SmallVec::new(),
                event_subs: SmallVec::new(),
                locked: false,
                name,
                location,
            });
            rt.owners
                .get_mut(owner)
                .expect("current owner slot missing")
                .entities
                .push(key);
            key
        });
        let value = {
            let mut cx = EntityCx::<'_, T>::new_unchecked(key);
            build(&mut cx)
        };
        with_rt(|rt| {
            if let Some(slot) = rt.entities.get_mut(key) {
                slot.value = Box::new(value);
            }
        });
        drain_pending_observers();
        Self::from_key(key)
    }

    #[must_use]
    pub const fn id(self) -> EntityId {
        EntityId(self.key)
    }

    #[track_caller]
    pub fn update<R>(self, f: impl FnOnce(&mut T, &mut EntityCx<'_, T>) -> R) -> R {
        let caller = Location::caller();
        run_update::<T, R>(self.key, caller, f)
    }

    #[track_caller]
    pub fn read<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let caller = Location::caller();
        let value = with_rt(|rt| {
            let Some(slot) = rt.entities.get_mut(self.key) else {
                crate::reactive::diagnostic::disposed_entity("read", caller);
            };
            assert!(
                !slot.locked,
                "Entity::read while update closure is still running (at {caller})"
            );
            core::mem::replace(&mut slot.value, Box::new(())) as Box<dyn core::any::Any>
        });
        let mut guard = ReadGuard {
            key: self.key,
            value: Some(value),
        };
        let inner = guard
            .value
            .as_ref()
            .expect("populated by guard ctor")
            .downcast_ref::<T>()
            .expect("entity type mismatch");
        let result = f(inner);
        let restored = guard.value.take().expect("value present");
        with_rt(|rt| {
            if let Some(slot) = rt.entities.get_mut(self.key) {
                slot.value = restored;
            }
        });
        result
    }
}

struct ReadGuard {
    key: EntityKey,
    value: Option<Box<dyn core::any::Any>>,
}

impl Drop for ReadGuard {
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

#[track_caller]
pub(crate) fn run_update<T: 'static, R>(
    key: EntityKey,
    caller: &'static Location<'static>,
    f: impl FnOnce(&mut T, &mut EntityCx<'_, T>) -> R,
) -> R {
    let (taken, prev_entity) = with_rt(|rt| {
        crate::reactive::phase::assert_paint_safe(rt, "Entity::update");
        let Some(slot) = rt.entities.get_mut(key) else {
            crate::reactive::diagnostic::disposed_entity("update", caller);
        };
        assert!(
            !slot.locked,
            "Entity::update called recursively on the same entity (at {caller})"
        );
        slot.locked = true;
        let taken = core::mem::replace(&mut slot.value, Box::new(())) as Box<dyn core::any::Any>;
        let prev = rt.current_entity.replace(key);
        rt.update_depth += 1;
        (taken, prev)
    });
    let mut guard = UpdateGuard {
        key,
        value: Some(taken),
        prev_entity,
    };
    let result;
    {
        let inner = guard
            .value
            .as_mut()
            .expect("populated by guard ctor")
            .downcast_mut::<T>()
            .expect("entity type mismatch");
        let mut cx = EntityCx::<'_, T>::new_unchecked(key);
        result = f(inner, &mut cx);
    }
    let restored = guard
        .value
        .take()
        .expect("value present after normal return");
    let is_outermost = with_rt(|rt| {
        if let Some(slot) = rt.entities.get_mut(key) {
            slot.value = restored;
            slot.locked = false;
        }
        rt.current_entity = guard.prev_entity;
        rt.update_depth -= 1;
        rt.update_depth == 0
    });
    core::mem::forget(guard);
    if is_outermost {
        drain_pending_observers();
    }
    result
}

struct UpdateGuard {
    key: EntityKey,
    value: Option<Box<dyn core::any::Any>>,
    prev_entity: Option<EntityKey>,
}

impl Drop for UpdateGuard {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            with_rt(|rt| {
                if let Some(slot) = rt.entities.get_mut(self.key) {
                    slot.value = value;
                    slot.locked = false;
                }
                rt.current_entity = self.prev_entity;
                if rt.update_depth > 0 {
                    rt.update_depth -= 1;
                }
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub(crate) EntityKey);
