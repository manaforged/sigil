use core::cell::RefCell;
use core::panic::Location;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{BTreeSet, VecDeque};

use crate::reactive::arena::Arena;
use crate::reactive::phase::DrawPhase;
pub(crate) use crate::reactive::runtime_slots::{
    EffectKey, EffectSlot, OwnerKey, OwnerSlot, SignalKey, SignalSlot,
};

static RUNTIME_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct RuntimeState {
    pub(crate) owners: Arena<OwnerSlot>,
    pub(crate) signals: Arena<SignalSlot>,
    pub(crate) effects: Arena<EffectSlot>,
    pub(crate) current_owner: Option<OwnerKey>,
    pub(crate) current_effect: Option<EffectKey>,
    pub(crate) batch_depth: u32,
    pub(crate) pending: VecDeque<EffectKey>,
    pub(crate) pending_set: BTreeSet<EffectKey>,
    pub(crate) running: BTreeSet<EffectKey>,
    pub(crate) dispose_after_run: Vec<EffectKey>,
    pub(crate) deferred_cleanups: Vec<Box<dyn FnOnce()>>,
    pub(crate) panic_origin: Option<(EffectKey, &'static str, &'static Location<'static>)>,
    pub(crate) phase: DrawPhase,
    pub(crate) frame_dirty: bool,
    pub(crate) runtime_id: u64,
}

impl RuntimeState {
    const fn new() -> Self {
        Self {
            owners: Arena::new(),
            signals: Arena::new(),
            effects: Arena::new(),
            current_owner: None,
            current_effect: None,
            batch_depth: 0,
            pending: VecDeque::new(),
            pending_set: BTreeSet::new(),
            running: BTreeSet::new(),
            dispose_after_run: Vec::new(),
            deferred_cleanups: Vec::new(),
            panic_origin: None,
            phase: DrawPhase::Idle,
            frame_dirty: true,
            runtime_id: 0,
        }
    }
}

thread_local! {
    static RUNTIME: RefCell<RuntimeState> = const { RefCell::new(RuntimeState::new()) };
}

#[inline]
pub(crate) fn with_rt<R>(f: impl FnOnce(&mut RuntimeState) -> R) -> R {
    RUNTIME.with(|rt| f(&mut rt.borrow_mut()))
}

pub(crate) fn runtime_id() -> u64 {
    with_rt(|rt| {
        if rt.runtime_id == 0 {
            rt.runtime_id = RUNTIME_ID_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        }
        rt.runtime_id
    })
}

#[derive(Debug)]
pub struct Runtime;

impl Runtime {
    #[track_caller]
    pub fn scope<R>(name: &'static str, f: impl FnOnce() -> R) -> R {
        let owner = crate::reactive::owner::Owner::new(name, None);
        let result = owner.with(f);
        owner.dispose();
        result
    }

    #[must_use]
    pub fn current_phase() -> DrawPhase {
        with_rt(|rt| rt.phase)
    }
}
