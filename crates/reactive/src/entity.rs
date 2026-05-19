//! Reactive entities — `Entity<T>` and `EntityId`.
//!
//! `Entity<T>` is a `Copy` arena handle to a piece of user state
//! owned by the reactive runtime, observed by other entities, and
//! disposed when its owner scope ends. See DR-045.
//!
//! Write path (`Entity::update`) and observer drain wiring live in
//! [`crate::entity_observer`]; this module owns the handle type plus
//! the read/build paths.

use core::marker::PhantomData;
use core::panic::Location;

use smallvec::SmallVec;

use crate::cx::Context;
use crate::entity_observer::drain_pending_observers;
use crate::runtime::{EntityKey, EntitySlot, with_rt};

/// A typed, `Copy` handle to a piece of user state owned by the
/// reactive runtime.
///
/// `Entity<T>` is an arena handle, not the value itself. Holding an
/// `Entity<T>` after its owner has been disposed is safe but
/// `update` / `read` will panic with a stale-handle message.
///
/// Entities are `!Send + !Sync` for the same reason `Signal<T>` is:
/// the arena lives in thread-local storage and `T` may not itself be
/// `Send`.
pub struct Entity<T: 'static> {
    pub(crate) key: EntityKey,
    // !Send + !Sync because the arena is thread-local and values
    // may not themselves be `Send`.
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
    /// Wrap a raw arena key as an `Entity<T>`. `pub(crate)` so global
    /// helpers can resurrect entities from a `TypeId` lookup.
    pub(crate) const fn from_key(key: EntityKey) -> Self {
        Self {
            key,
            _not_send: PhantomData,
        }
    }

    /// Create a new entity in the current reactive owner scope.
    ///
    /// `build` runs once with a `Context<'_, T>` whose `T` payload is
    /// not yet installed — `cx.notify()` inside `build` is a no-op
    /// because no observers have registered yet. The returned value
    /// becomes the entity's payload.
    ///
    /// # Panics
    ///
    /// Panics if called outside any reactive scope. Use
    /// [`Runtime::scope`](crate::Runtime::scope) or
    /// [`Owner::with`](crate::Owner::with) to establish one.
    #[must_use]
    #[track_caller]
    pub fn new(name: &'static str, build: impl FnOnce(&mut Context<'_, T>) -> T) -> Self {
        let location = Location::caller();
        let key = with_rt(|rt| {
            let owner = rt
                .current_owner
                .unwrap_or_else(|| crate::diagnostic::no_scope("Entity::new()", location));
            assert!(
                rt.owners.contains(owner),
                "Entity::new() with a disposed current owner (at {location})"
            );
            let key = rt.entities.insert(EntitySlot {
                owner,
                // Sentinel: `()` lives in the slot until `build`
                // returns. Downcasts to `T` reject this gracefully
                // because no public path can read the slot while
                // build is in flight (build holds the context).
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
            // The build closure receives a Context bound to this
            // entity. notify() inside build is a no-op (no
            // subscribers) but cx.observe()/cx.subscribe() may run.
            let mut cx = Context::<'_, T>::new_unchecked(key);
            build(&mut cx)
        };
        with_rt(|rt| {
            if let Some(slot) = rt.entities.get_mut(key) {
                slot.value = Box::new(value);
            }
        });
        // Drain any observers queued during build (e.g. parent
        // entities that notify-during-init). At top-level scope this
        // is the outermost update so we must drain to honor the
        // batched-notify semantics.
        drain_pending_observers();
        Self::from_key(key)
    }

    /// This entity's stable identity.
    ///
    /// `EntityId` is the type-erased form of `Entity<T>`; useful for
    /// keying observer maps that hold heterogeneous entities.
    #[must_use]
    pub const fn id(self) -> EntityId {
        EntityId(self.key)
    }

    /// Mutate this entity through a closure.
    ///
    /// The closure receives `&mut T` and a `Context<'_, T>` borrow.
    /// `cx.notify()` inside the closure schedules every observer of
    /// this entity for the end-of-turn drain. Re-entrant `update` on
    /// the *same* entity panics with "entity already locked for
    /// update"; updating a *different* entity from inside is allowed.
    ///
    /// # Panics
    ///
    /// Panics if the entity has been disposed, or if this entity is
    /// already inside a live `update` on this thread.
    #[track_caller]
    pub fn update<R>(self, f: impl FnOnce(&mut T, &mut Context<'_, T>) -> R) -> R {
        let caller = Location::caller();
        run_update::<T, R>(self.key, caller, f)
    }

    /// Read-only access to this entity's payload.
    ///
    /// Like `Signal::peek`, this does not subscribe the current
    /// effect — observation goes through `Context::observe`, not
    /// through `Entity::read`.
    ///
    /// # Panics
    ///
    /// Panics if the entity has been disposed.
    #[track_caller]
    pub fn read<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let caller = Location::caller();
        // Take the value out so user code cannot trigger a re-entrant
        // borrow through `with_rt`. The slot keeps an `UpdateInProgress`
        // sentinel via the locked flag.
        let value = with_rt(|rt| {
            let Some(slot) = rt.entities.get_mut(self.key) else {
                crate::diagnostic::disposed_entity("read", caller);
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
        // Restore explicitly so guard's drop is a no-op on normal
        // return (the take() empties the option).
        let restored = guard.value.take().expect("value present");
        with_rt(|rt| {
            if let Some(slot) = rt.entities.get_mut(self.key) {
                slot.value = restored;
            }
        });
        result
    }
}

/// RAII restore guard for `Entity::read` — puts the value back if
/// `f` panics. On normal return the caller takes the value out and
/// the drop is a no-op.
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

/// Run the update closure with the entity's locked flag set, restore
/// the payload, and drain pending observers if this is the outermost
/// update. Lives at module scope so `Context::new_entity` can reuse
/// it.
#[track_caller]
pub(crate) fn run_update<T: 'static, R>(
    key: EntityKey,
    caller: &'static Location<'static>,
    f: impl FnOnce(&mut T, &mut Context<'_, T>) -> R,
) -> R {
    let (taken, prev_entity) = with_rt(|rt| {
        crate::phase::assert_paint_safe(rt, "Entity::update");
        let Some(slot) = rt.entities.get_mut(key) else {
            crate::diagnostic::disposed_entity("update", caller);
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
        let mut cx = Context::<'_, T>::new_unchecked(key);
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
    // Suppress drop's restore path now that we have restored.
    core::mem::forget(guard);
    if is_outermost {
        drain_pending_observers();
    }
    result
}

/// RAII restore for `Entity::update` — runs only on unwind.
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

/// Type-erased entity identity.
///
/// Carries the generational arena key without the `T` phantom. Used
/// as a key in observer maps and action-dispatch tables (G3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub(crate) EntityKey);
