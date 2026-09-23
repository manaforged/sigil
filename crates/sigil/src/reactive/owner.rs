use core::panic::Location;

use crate::reactive::owner_dispose::dispose_owner;
use crate::reactive::runtime::{OwnerKey, OwnerSlot, with_rt};

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
            let index_in_parent =
                parent_key.and_then(|pk| rt.owners.get(pk).map(|p| p.children.len()));
            let key = rt.owners.insert(OwnerSlot {
                parent: parent_key,
                index_in_parent,
                children: Vec::new(),
                signals: Vec::new(),
                effects: Vec::new(),
                cleanups: Vec::new(),
                name,
                location,
            });
            if let Some(pk) = parent_key
                && let Some(p) = rt.owners.get_mut(pk)
            {
                p.children.push(key);
            }
            key
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
        let prev = with_rt(|rt| {
            assert!(
                rt.owners.contains(self.id.0),
                "Owner::with on a disposed owner (id={:?}, at {location})",
                self.id.0,
            );
            let previous = rt.current_owner;
            rt.current_owner = Some(self.id.0);
            previous
        });
        let _guard = OwnerWithGuard { prev };
        f()
    }

    pub fn dispose(self) {
        dispose_owner(self.id.0);
    }
}

#[must_use]
pub fn is_live(id: OwnerId) -> bool {
    with_rt(|rt| rt.owners.contains(id.0))
}

#[track_caller]
pub fn owner_by_id(id: OwnerId, f: impl FnOnce()) -> bool {
    let prev = with_rt(|rt| {
        if !rt.owners.contains(id.0) {
            return None;
        }
        let previous = rt.current_owner;
        rt.current_owner = Some(id.0);
        Some(previous)
    });
    let Some(prev) = prev else {
        return false;
    };
    let _guard = OwnerWithGuard { prev };
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
