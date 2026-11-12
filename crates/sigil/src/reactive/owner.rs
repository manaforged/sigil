use core::panic::Location;

use crate::reactive::owner_dispose::dispose_owner;
use crate::reactive::runtime::{EffectKey, OwnerKey, OwnerSlot, RuntimeState, with_rt};

impl RuntimeState {
    pub(crate) fn insert_owner(
        &mut self,
        #[cfg_attr(not(feature = "kittest"), allow(unused_variables))] name: &'static str,
        parent: Option<OwnerKey>,
        #[cfg_attr(not(feature = "kittest"), allow(unused_variables))] location: &'static Location<
            'static,
        >,
    ) -> OwnerKey {
        let index_in_parent = parent.and_then(|pk| self.owners.get(pk).map(|p| p.children.len()));
        let key = self.owners.insert(OwnerSlot {
            parent,
            index_in_parent,
            children: Vec::new(),
            signals: Vec::new(),
            effects: Vec::new(),
            cleanups: Vec::new(),
            #[cfg(feature = "kittest")]
            name,
            #[cfg(feature = "kittest")]
            location,
        });
        if let Some(p) = parent.and_then(|pk| self.owners.get_mut(pk)) {
            p.children.push(key);
        }
        key
    }
}

struct OwnerWithGuard {
    prev: Option<OwnerKey>,
    prev_lazy: Option<EffectKey>,
    prev_deferred: Option<usize>,
}

impl Drop for OwnerWithGuard {
    fn drop(&mut self) {
        with_rt(|rt| {
            rt.current_owner = self.prev;
            rt.lazy_effect_owner = self.prev_lazy;
            rt.deferred_at = self.prev_deferred;
        });
    }
}

pub(crate) struct Deferred {
    prev: Option<OwnerKey>,
    prev_lazy: Option<EffectKey>,
    outer: Option<usize>,
    fallback: OwnerKey,
    owner: Option<OwnerKey>,
    name: &'static str,
    location: &'static Location<'static>,
}

impl RuntimeState {
    pub(crate) fn materialize(&mut self, at: usize) -> OwnerKey {
        let Some(entry) = self.deferred.get(at) else {
            unreachable!("invariant: a deferred owner index points into the deferred stack");
        };
        if let Some(owner) = entry.owner {
            return owner;
        }
        let (prev, prev_lazy, outer, fallback, name, location) = (
            entry.prev,
            entry.prev_lazy,
            entry.outer,
            entry.fallback,
            entry.name,
            entry.location,
        );
        let parent = if let Some(owner) = prev {
            owner
        } else if let Some(outer) = outer {
            self.materialize(outer)
        } else if let Some(run_owner) = prev_lazy.and_then(|key| self.effect_run_owner(key)) {
            if let Some(slot) = self.deferred.get_mut(at) {
                slot.prev = Some(run_owner);
                slot.prev_lazy = None;
            }
            run_owner
        } else {
            fallback
        };
        assert!(
            self.owners.contains(parent),
            "Owner::new with a disposed parent (id={parent:?}, creating {name:?} at {location})",
        );
        let owner = self.insert_owner(name, Some(parent), location);
        if let Some(slot) = self.deferred.get_mut(at) {
            slot.owner = Some(owner);
        }
        owner
    }

    fn settled_owner(&self) -> Option<OwnerKey> {
        let mut at = match self.current_owner {
            Some(owner) => return Some(owner),
            None => self.deferred_at?,
        };
        loop {
            let entry = self.deferred.get(at)?;
            if let Some(owner) = entry.owner.or(entry.prev) {
                return Some(owner);
            }
            let Some(outer) = entry.outer else {
                let effect = entry.prev_lazy.and_then(|key| self.effects.get(key));
                return Some(effect.map_or(entry.fallback, |slot| slot.owner));
            };
            at = outer;
        }
    }
}

/// An owner that is created only when the scope it covers asks for the current owner.
pub(crate) struct DeferredOwner(usize);

impl DeferredOwner {
    pub(crate) fn finish(self) -> Option<Owner> {
        with_rt(|rt| rt.deferred.get(self.0).and_then(|entry| entry.owner)).map(Owner)
    }
}

impl Drop for DeferredOwner {
    fn drop(&mut self) {
        with_rt(|rt| {
            rt.deferred.truncate(self.0 + 1);
            if let Some(entry) = rt.deferred.pop() {
                rt.current_owner = entry.prev;
                rt.lazy_effect_owner = entry.prev_lazy;
                rt.deferred_at = entry.outer;
            }
        });
    }
}

/// Reactive scope that owns effects, signals, and cleanups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Owner(pub(crate) OwnerKey);

impl Owner {
    /// Creates an owner.
    #[must_use]
    #[track_caller]
    pub fn new(name: &'static str, parent: Option<Self>) -> Self {
        let location = Location::caller();
        let parent_key = parent.map(|p| p.0);
        let key = with_rt(|rt| {
            if let Some(pk) = parent_key {
                assert!(
                    rt.owners.contains(pk),
                    "Owner::new with a disposed parent (id={pk:?}, creating {name:?} at {location})",
                );
            }
            rt.insert_owner(name, parent_key, location)
        });
        Self(key)
    }

    /// Runs `f` in a new named owner, then disposes the owner.
    #[track_caller]
    pub fn scope<R>(name: &'static str, f: impl FnOnce() -> R) -> R {
        let owner = Self::new(name, None);
        let result = owner.with(f);
        owner.dispose();
        result
    }

    #[track_caller]
    pub(crate) fn defer(name: &'static str, fallback: Self) -> DeferredOwner {
        let location = Location::caller();
        with_rt(|rt| {
            let at = rt.deferred.len();
            let entry = Deferred {
                prev: rt.current_owner.take(),
                prev_lazy: rt.lazy_effect_owner.take(),
                outer: rt.deferred_at.replace(at),
                fallback: fallback.0,
                owner: None,
                name,
                location,
            };
            rt.deferred.push(entry);
            DeferredOwner(at)
        })
    }

    pub(crate) fn settled() -> Option<Self> {
        with_rt(|rt| rt.settled_owner().map(Self))
    }

    /// Returns the current owner, if any.
    #[must_use]
    pub fn current() -> Option<Self> {
        with_rt(|rt| rt.active_owner().map(Self))
    }

    /// Returns whether this owner is alive.
    #[must_use]
    pub fn is_alive(self) -> bool {
        is_live(self)
    }

    /// Runs `f` with this owner as the current owner.
    #[track_caller]
    pub fn with<R>(self, f: impl FnOnce() -> R) -> R {
        let location = Location::caller();
        let guard = enter(self);
        assert!(
            guard.is_some(),
            "Owner::with on a disposed owner (id={:?}, at {location})",
            self.0,
        );
        f()
    }

    /// Drops the effects, tasks, and child owners under this owner.
    pub fn dispose(self) {
        dispose_owner(self.0);
    }
}

#[must_use]
pub(crate) fn is_live(id: Owner) -> bool {
    with_rt(|rt| rt.owners.contains(id.0))
}

fn enter(id: Owner) -> Option<OwnerWithGuard> {
    with_rt(|rt| {
        rt.owners.contains(id.0).then(|| OwnerWithGuard {
            prev: rt.current_owner.replace(id.0),
            prev_lazy: rt.lazy_effect_owner.take(),
            prev_deferred: rt.deferred_at.take(),
        })
    })
}

#[track_caller]
pub(crate) fn owner_by_id(id: Owner, f: impl FnOnce()) -> bool {
    let Some(_guard) = enter(id) else {
        return false;
    };
    f();
    true
}

/// Run `f` when the current owner is disposed.
#[track_caller]
pub fn cleanup(f: impl FnOnce() + 'static) {
    let f: Box<dyn FnOnce()> = Box::new(f);
    with_rt(|rt| {
        let owner = rt
            .active_owner()
            .expect("cleanup called outside a reactive scope");
        rt.owners
            .get_mut(owner)
            .expect("current owner slot missing")
            .cleanups
            .push(f);
    });
}

#[cfg(test)]
#[path = "owner_tests.rs"]
mod tests;
