//! TypeId-keyed global entities.
//!
//! Globals are entities under the hood: a `set_global::<G>(value)`
//! installs an `Entity<G>` parented to the current owner and indexes
//! it in `RuntimeState::globals: Vec<(TypeId, EntityKey)>`. Observers
//! get the same notify path, dispose cascade, and Subscription
//! semantics as entity observers. See DR-045 §6.

use core::any::TypeId;
use core::panic::Location;

use smallvec::SmallVec;

use crate::entity::Entity;
use crate::entity_observer::{Subscription, register_observe_global};
use crate::runtime::{EntityKey, EntitySlot, PendingObserver, with_rt};

/// Look up the global of type `G` if installed.
pub(crate) fn lookup_global<G: 'static>() -> Option<EntityKey> {
    let id = TypeId::of::<G>();
    with_rt(|rt| {
        rt.globals
            .iter()
            .find_map(|(k, key)| (*k == id).then_some(*key))
    })
}

/// Ensure a global entity exists for type `G`; lazily initialise with
/// `G::default()` if not. Returns the entity handle.
///
/// Used by `Context::observe_global` to allow observers to register
/// before the producer installs the value.
#[track_caller]
pub(crate) fn ensure_global_entity<G: 'static>() -> Entity<G> {
    if let Some(key) = lookup_global::<G>() {
        return Entity::<G>::from_key(key);
    }
    // No global yet — allocate a placeholder entity. The slot carries
    // a sentinel `()` payload; downcasts to `G` will fail until
    // `set_global::<G>(value)` installs the real payload. Observers
    // registered against the placeholder remain valid through the
    // install: the slot's EntityKey is reused.
    let location = Location::caller();
    let key = with_rt(|rt| {
        let owner = rt
            .current_owner
            .unwrap_or_else(|| crate::diagnostic::no_scope("observe_global", location));
        let key = rt.entities.insert(EntitySlot {
            owner,
            value: Box::new(()),
            subscribers: SmallVec::new(),
            event_subs: SmallVec::new(),
            locked: false,
            name: "<global-placeholder>",
            location,
        });
        rt.owners
            .get_mut(owner)
            .expect("current owner slot missing")
            .entities
            .push(key);
        rt.globals.push((TypeId::of::<G>(), key));
        key
    });
    Entity::<G>::from_key(key)
}

/// Install or replace the global of type `G`.
///
/// If `G` already has a global, the old entity is disposed (firing
/// its observers' on-drop via owner cascade) before the new payload
/// is installed at the same `EntityKey` slot — observers registered
/// against the placeholder remain attached and see the new value.
///
/// # Panics
///
/// Panics if called outside any reactive scope.
#[track_caller]
pub fn set_global<G: 'static>(value: G) {
    let location = Location::caller();
    let entity = ensure_global_entity::<G>();
    with_rt(|rt| {
        crate::phase::assert_paint_safe(rt, "set_global");
        let Some(slot) = rt.entities.get_mut(entity.key) else {
            crate::diagnostic::no_scope("set_global", location);
        };
        slot.value = Box::new(value);
        // Notify observers — first install of a global is a "change."
        let subs: SmallVec<[crate::runtime::ObserverKey; 4]> = slot.subscribers.clone();
        for observer in subs {
            rt.pending_observers.push_back(PendingObserver {
                observer,
                event: None,
            });
        }
    });
    crate::entity_observer::drain_pending_observers();
}

/// Read the global of type `G` without subscribing.
///
/// # Panics
///
/// Panics if `G` has no global installed in the current runtime.
#[track_caller]
pub fn global<G: 'static, R>(f: impl FnOnce(&G) -> R) -> R {
    let location = Location::caller();
    let key = lookup_global::<G>().unwrap_or_else(|| {
        crate::diagnostic::missing_global("global", core::any::type_name::<G>(), location);
    });
    Entity::<G>::from_key(key).read(f)
}

/// Mutate the global of type `G` and notify its observers.
///
/// # Panics
///
/// Panics if `G` has no global installed.
#[track_caller]
pub fn global_mut<G: 'static, R>(f: impl FnOnce(&mut G) -> R) -> R {
    let location = Location::caller();
    let key = lookup_global::<G>().unwrap_or_else(|| {
        crate::diagnostic::missing_global("global_mut", core::any::type_name::<G>(), location);
    });
    Entity::<G>::from_key(key).update(|g, cx| {
        let result = f(g);
        cx.notify();
        result
    })
}

/// Top-level observe-global outside any `Context`.
///
/// For consumers (effects, top-level setup) that want to observe a
/// global without first being inside an entity update. The callback
/// has no observer `T` — it is a bare `FnMut` that fires at the same
/// end-of-turn batch as entity observers.
#[track_caller]
pub fn observe_global<G: 'static>(callback: impl FnMut() + 'static) -> Subscription {
    let entity = ensure_global_entity::<G>();
    register_observe_global(entity.key, callback)
}
