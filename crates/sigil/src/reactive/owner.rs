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
}

impl Drop for OwnerWithGuard {
    fn drop(&mut self) {
        with_rt(|rt| {
            rt.current_owner = self.prev;
            rt.lazy_effect_owner = self.prev_lazy;
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
pub fn on_cleanup(f: impl FnOnce() + 'static) {
    let f: Box<dyn FnOnce()> = Box::new(f);
    with_rt(|rt| {
        let owner = rt
            .active_owner()
            .expect("on_cleanup called outside a reactive scope");
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
