use core::any::TypeId;
use core::panic::Location;

use smallvec::SmallVec;

use crate::reactive::entity::Entity;
use crate::reactive::entity_observer::{Subscription, register_observe_global};
use crate::reactive::runtime::{EntityKey, EntitySlot, PendingObserver, with_rt};

pub(crate) fn lookup_global<G: 'static>() -> Option<EntityKey> {
    let id = TypeId::of::<G>();
    with_rt(|rt| {
        rt.globals
            .iter()
            .find_map(|(k, key)| (*k == id).then_some(*key))
    })
}

#[track_caller]
pub(crate) fn ensure_global_entity<G: 'static>() -> Entity<G> {
    if let Some(key) = lookup_global::<G>() {
        return Entity::<G>::from_key(key);
    }
    let location = Location::caller();
    let key = with_rt(|rt| {
        let owner = rt
            .current_owner
            .unwrap_or_else(|| crate::reactive::diagnostic::no_scope("observe_global", location));
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

#[track_caller]
pub fn set_global<G: 'static>(value: G) {
    let location = Location::caller();
    let entity = ensure_global_entity::<G>();
    with_rt(|rt| {
        crate::reactive::phase::assert_paint_safe(rt, "set_global");
        let Some(slot) = rt.entities.get_mut(entity.key) else {
            crate::reactive::diagnostic::no_scope("set_global", location);
        };
        slot.value = Box::new(value);
        let subs: SmallVec<[crate::reactive::runtime::ObserverKey; 4]> = slot.subscribers.clone();
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
    crate::reactive::entity_observer::drain_pending_observers();
}

#[track_caller]
pub fn global<G: 'static, R>(f: impl FnOnce(&G) -> R) -> R {
    let location = Location::caller();
    let key = lookup_global::<G>().unwrap_or_else(|| {
        crate::reactive::diagnostic::missing_global(
            "global",
            core::any::type_name::<G>(),
            location,
        );
    });
    Entity::<G>::from_key(key).read(f)
}

#[track_caller]
pub fn global_mut<G: 'static, R>(f: impl FnOnce(&mut G) -> R) -> R {
    let location = Location::caller();
    let key = lookup_global::<G>().unwrap_or_else(|| {
        crate::reactive::diagnostic::missing_global(
            "global_mut",
            core::any::type_name::<G>(),
            location,
        );
    });
    Entity::<G>::from_key(key).update(|g, cx| {
        let result = f(g);
        cx.notify();
        result
    })
}

#[track_caller]
pub fn observe_global<G: 'static>(callback: impl FnMut() + 'static) -> Subscription {
    let entity = ensure_global_entity::<G>();
    register_observe_global(entity.key, callback)
}
