use core::any::Any;
use core::panic::Location;

use smallvec::SmallVec;

use crate::reactive::arena::Key;

pub(crate) type OwnerKey = Key<OwnerSlot>;
pub(crate) type SignalKey = Key<SignalSlot>;
pub(crate) type EffectKey = Key<EffectSlot>;

pub(crate) struct OwnerSlot {
    pub(crate) parent: Option<OwnerKey>,
    pub(crate) index_in_parent: Option<usize>,
    pub(crate) children: Vec<OwnerKey>,
    pub(crate) signals: Vec<SignalKey>,
    pub(crate) effects: Vec<EffectKey>,
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
