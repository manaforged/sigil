use core::panic::Location;

use crate::reactive::owner_dispose::dispose_owner;
use crate::reactive::runtime::{OwnerKey, OwnerSlot, RuntimeState, with_rt};

impl RuntimeState {
    pub(crate) fn insert_owner(
        &mut self,
        name: &'static str,
        parent: Option<OwnerKey>,
        location: &'static Location<'static>,
    ) -> OwnerKey {
        let index_in_parent = parent.and_then(|pk| self.owners.get(pk).map(|p| p.children.len()));
        let key = self.owners.insert(OwnerSlot {
            parent,
            index_in_parent,
            children: Vec::new(),
            signals: Vec::new(),
            effects: Vec::new(),
            cleanups: Vec::new(),
            name,
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
}

impl Drop for OwnerWithGuard {
    fn drop(&mut self) {
        with_rt(|rt| rt.current_owner = self.prev);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OwnerId(pub(crate) OwnerKey);

#[derive(Debug, Clone, Copy)]
pub struct Owner {
    id: OwnerId,
    _not_send: core::marker::PhantomData<*const ()>,
}

impl Owner {
    #[must_use]
    #[track_caller]
    pub fn new(name: &'static str, parent: Option<OwnerId>) -> Self {
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
        Self {
            id: OwnerId(key),
            _not_send: core::marker::PhantomData,
        }
    }

    #[must_use]
    pub const fn id(self) -> OwnerId {
        self.id
    }

    #[must_use]
    pub fn current() -> Option<OwnerId> {
        with_rt(|rt| rt.current_owner.map(OwnerId))
    }

    #[track_caller]
    pub fn with<R>(self, f: impl FnOnce() -> R) -> R {
        let location = Location::caller();
        let guard = enter(self.id);
        assert!(
            guard.is_some(),
            "Owner::with on a disposed owner (id={:?}, at {location})",
            self.id.0,
        );
        f()
    }

    pub fn dispose(self) {
        dispose_owner(self.id.0);
    }
}

#[must_use]
pub(crate) fn is_live(id: OwnerId) -> bool {
    with_rt(|rt| rt.owners.contains(id.0))
}

fn enter(id: OwnerId) -> Option<OwnerWithGuard> {
    with_rt(|rt| {
        rt.owners.contains(id.0).then(|| OwnerWithGuard {
            prev: rt.current_owner.replace(id.0),
        })
    })
}

#[track_caller]
pub(crate) fn owner_by_id(id: OwnerId, f: impl FnOnce()) -> bool {
    let Some(_guard) = enter(id) else {
        return false;
    };
    f();
    true
}

#[track_caller]
pub fn on_cleanup(f: impl FnOnce() + 'static) {
    let f: Box<dyn FnOnce()> = Box::new(f);
    with_rt(|rt| {
        let owner = rt
            .current_owner
            .expect("on_cleanup called outside a reactive scope");
        rt.owners
            .get_mut(owner)
            .expect("current owner slot missing")
            .cleanups
            .push(f);
    });
}
