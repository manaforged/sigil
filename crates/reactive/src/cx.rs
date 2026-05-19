//! `Context<'_, T>` — the borrow handed to entity update closures and
//! observer callbacks.
//!
//! DR-045 deviation: the spec sketches `Context<'a, T>` as carrying a
//! `&'a mut RuntimeState`. Holding the runtime borrow across user code
//! conflicts with the existing `with_rt` re-entrancy discipline — every
//! reactive primitive in this crate (`Signal::set`, `effect`, `batch`,
//! `Owner::with`) routes through `with_rt(|rt| ...)`, taking a fresh
//! mutable borrow each time. A `Context` that holds `&mut RuntimeState`
//! would force every user-facing op to thread that borrow through, or
//! deadlock the thread-local `RefCell` on the first inner call. Instead
//! we make `Context<'a, T>` a borrow-checked token: it carries the
//! `EntityKey` of the entity it is "inside," plus the lifetime + `T`
//! phantom that the call-site bytecode needs to enforce
//! "single-active-context-per-update-closure." Methods on `Context`
//! call `with_rt` themselves, which is the same pattern `Signal<T>`
//! uses. Followup: capture this in a DR-045 amendment.
//!
//! See DR-045 §7 (Context shape) and `docs/specs/g2-entity-api-sketch.md`.
//!
//! The `impl Context` block is split across sibling modules to keep
//! this file under the 400-line cap:
//!
//! - [`cx_actions`](crate::cx_actions) — `action`,
//!   `dispatch`, plus the deferred-replay seam.
//! - [`cx_focus`](crate::cx_focus) — focus-stack methods
//!   (`focus_handle`, `focus`, `blur`, `focused`, `painting`).
//! - [`cx_spawn`](crate::cx_spawn) — `spawn` and the `spawn_through`
//!   monomorphisation seam.

use core::any::{Any, TypeId};
use core::marker::PhantomData;
use core::panic::Location;

use smallvec::SmallVec;

use crate::entity::Entity;
use crate::entity_observer::{Subscription, register_observe, register_subscribe};
use crate::owner::on_cleanup;
use crate::runtime::{EntityKey, ObserverKey, PendingObserver, with_rt};

/// The borrow handed to entity update closures and observer callbacks.
///
/// Carries the identity of the "self" entity — the entity whose
/// `update` is currently active, or the observer's own entity inside
/// a callback — and a lifetime + `T` phantom that enforces single-use
/// inside the closure. Methods route through `with_rt`, matching the
/// rest of the crate.
pub struct Context<'a, T: 'static> {
    pub(crate) entity: EntityKey,
    _life: PhantomData<&'a mut T>,
    // !Send + !Sync — runtime is thread-local.
    _not_send: PhantomData<*const ()>,
}

impl<T: 'static> Context<'_, T> {
    /// Internal constructor — wraps an `EntityKey` as a context token.
    /// `pub(crate)` so `Entity::new`, `Entity::update`, and observer
    /// drain can mint a context for their entity.
    pub(crate) const fn new_unchecked(entity: EntityKey) -> Self {
        Self {
            entity,
            _life: PhantomData,
            _not_send: PhantomData,
        }
    }

    /// Notify observers of `self`.
    ///
    /// Schedules every observer of the current entity for the
    /// end-of-turn drain. Safe to call multiple times in one closure —
    /// per-observer dedup happens at drain time via the `pending_set`
    /// equivalent on the entity-observer queue.
    pub fn notify(&mut self) {
        let key = self.entity;
        with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "Context::notify");
            let Some(slot) = rt.entities.get(key) else {
                return;
            };
            let subs: SmallVec<[ObserverKey; 4]> = slot.subscribers.clone();
            for observer in subs {
                rt.pending_observers.push_back(PendingObserver {
                    observer,
                    event: None,
                });
            }
        });
    }

    /// Spawn a child entity in the current owner scope.
    ///
    /// Identical to top-level [`Entity::new`], reused for ergonomic
    /// parity with the GPUI surface.
    #[track_caller]
    pub fn entity<U: 'static>(
        &mut self,
        name: &'static str,
        build: impl FnOnce(&mut Context<'_, U>) -> U,
    ) -> Entity<U> {
        Entity::<U>::new(name, build)
    }

    /// Observe an entity for any `notify()` it fires.
    ///
    /// Returns a `Subscription` whose drop severs the observation.
    /// Callbacks run on the main thread, batched at end-of-turn.
    #[track_caller]
    pub fn observe<U: 'static>(
        &mut self,
        observed: &Entity<U>,
        callback: impl FnMut(&mut T, &Entity<U>, &mut Context<'_, T>) + 'static,
    ) -> Subscription {
        register_observe::<T, U>(self.entity, *observed, callback)
    }

    /// Typed event subscription on an entity.
    ///
    /// Sub-channel of `observe` parameterized by `Ev`. The observed
    /// entity emits events via `Context::emit::<Ev>` inside its own
    /// `update`; subscribers receive the event payload.
    #[track_caller]
    pub fn subscribe<U: 'static, Ev: 'static>(
        &mut self,
        observed: &Entity<U>,
        callback: impl FnMut(&mut T, &Entity<U>, &Ev, &mut Context<'_, T>) + 'static,
    ) -> Subscription {
        register_subscribe::<T, U, Ev>(self.entity, *observed, callback)
    }

    /// Emit a typed event from inside `self`'s `update`.
    ///
    /// Schedules every `subscribe::<T, Ev>` observer for end-of-turn
    /// drain with `ev` as the payload. The payload is cloned to each
    /// subscriber except the last, which receives the original — this
    /// matches the GPUI / Dioxus / Bevy full-broadcast contract.
    ///
    /// `Ev: Clone` is bounded so the broadcast can hand every observer
    /// its own copy without resorting to a shared `Rc`. For events
    /// that are expensive to clone, wrap them in `Rc` / `Arc` at the
    /// call site.
    pub fn emit<Ev: Clone + 'static>(&mut self, ev: Ev) {
        let key = self.entity;
        let ev_type = TypeId::of::<Ev>();
        let mut observers: SmallVec<[ObserverKey; 4]> = with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "Context::emit");
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
        // Broadcast: clone the payload to every subscriber except the
        // last, which receives the original `ev`. This preserves the
        // full-payload guarantee for N subscribers while avoiding a
        // clone in the common single-subscriber case.
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

    /// Observe a global of type `G`.
    ///
    /// Equivalent to `observe(&global_entity::<G>())` but spelled to
    /// match GPUI's `observe_global`. Returns a `Subscription`.
    #[track_caller]
    pub fn observe_global<G: 'static>(
        &mut self,
        mut callback: impl FnMut(&mut T, &mut Context<'_, T>) + 'static,
    ) -> Subscription {
        let entity = crate::entity_global::ensure_global_entity::<G>();
        // Adapter: the underlying observer signature passes the
        // observed handle; globals are keyed by TypeId, not handle, so
        // the user-facing callback drops that arg.
        register_observe::<T, G>(self.entity, entity, move |this, _g, cx| {
            callback(this, cx);
        })
    }

    /// Mutable access to the entity this context is inside.
    ///
    /// Deviation note: returning `&mut T` from `Context` would require
    /// holding the arena borrow live across the `&mut T`, which the
    /// thread-local `RefCell` cannot express. We expose `mutate`
    /// instead as a closure-shaped helper that runs `f(&mut T)` with
    /// the payload locked. See followups.
    ///
    /// # Panics
    ///
    /// Panics if the entity has been disposed.
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

    /// Cleanup hook tied to this context's owner.
    ///
    /// Routes to the top-level `on_cleanup`; the current owner is the
    /// active owner when the closure runs, which during `Entity::new`
    /// or `Entity::update` is the entity's owner.
    pub fn on_cleanup(&mut self, f: impl FnOnce() + 'static) {
        on_cleanup(f);
    }
}

/// Mutable-reborrow guard — restores payload on unwind.
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
