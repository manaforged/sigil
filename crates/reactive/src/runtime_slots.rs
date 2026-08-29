#![allow(
    dead_code,
    reason = "entity / observer plumbing consumed by sibling modules in this G2 landing"
)]

use core::any::{Any, TypeId};
use core::panic::Location;

use smallvec::SmallVec;

use crate::arena::Key;

pub(crate) type OwnerKey = Key<OwnerSlot>;
pub(crate) type SignalKey = Key<SignalSlot>;
pub(crate) type EffectKey = Key<EffectSlot>;
pub(crate) type EntityKey = Key<EntitySlot>;
pub(crate) type ObserverKey = Key<ObserverSlot>;

pub(crate) struct PendingCompletion {
    pub(crate) owner: OwnerKey,
    pub(crate) callback: Box<dyn FnOnce() + Send + 'static>,
}

pub(crate) struct EntitySlot {
    pub(crate) owner: OwnerKey,
    pub(crate) value: Box<dyn Any>,
    pub(crate) subscribers: SmallVec<[ObserverKey; 4]>,
    pub(crate) event_subs: SmallVec<[(TypeId, ObserverKey); 4]>,
    pub(crate) locked: bool,
    pub(crate) name: &'static str,
    pub(crate) location: &'static Location<'static>,
}

pub(crate) type ObserveFn = Box<dyn FnMut(ObserverKey)>;
pub(crate) type ObserveBareFn = Box<dyn FnMut()>;
pub(crate) type SubscribeFn = Box<dyn FnMut(ObserverKey, &dyn Any)>;

pub(crate) enum ObserverKind {
    Observe(ObserveFn),
    ObserveBare(ObserveBareFn),
    Subscribe { ev_type: TypeId, f: SubscribeFn },
}

pub(crate) struct ObserverSlot {
    pub(crate) observer_entity: Option<EntityKey>,
    pub(crate) observed: EntityKey,
    pub(crate) owner: OwnerKey,
    pub(crate) kind: ObserverKind,
}

pub(crate) struct PendingObserver {
    pub(crate) observer: ObserverKey,
    pub(crate) event: Option<Box<dyn Any>>,
}

pub(crate) struct OwnerSlot {
    pub(crate) parent: Option<OwnerKey>,
    pub(crate) index_in_parent: Option<usize>,
    pub(crate) children: Vec<OwnerKey>,
    pub(crate) signals: Vec<SignalKey>,
    pub(crate) effects: Vec<EffectKey>,
    pub(crate) entities: Vec<EntityKey>,
    pub(crate) observers: Vec<ObserverKey>,
    pub(crate) cleanups: Vec<Box<dyn FnOnce()>>,
    pub(crate) name: &'static str,
    pub(crate) location: &'static Location<'static>,
}

pub(crate) struct SignalSlot {
    #[expect(
        dead_code,
        reason = "construction-site owner; write path does not read it"
    )]
    pub(crate) owner: OwnerKey,
    pub(crate) value: Box<dyn Any>,
    pub(crate) subscribers: SmallVec<[EffectKey; 4]>,
    #[expect(
        dead_code,
        reason = "construction-site location; write path does not read it"
    )]
    pub(crate) location: &'static Location<'static>,
}

pub(crate) struct EffectSlot {
    pub(crate) owner: OwnerKey,
    pub(crate) run_owner: Option<OwnerKey>,
    pub(crate) f: Option<Box<dyn FnMut()>>,
    pub(crate) deps: SmallVec<[SignalKey; 4]>,
    pub(crate) prev_deps: SmallVec<[SignalKey; 4]>,
    pub(crate) prev_dep_cursor: usize,
    pub(crate) location: &'static Location<'static>,
    pub(crate) name: &'static str,
}
