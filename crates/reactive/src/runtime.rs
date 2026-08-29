#![allow(
    dead_code,
    reason = "entity / observer plumbing consumed by sibling modules in this G2 landing"
)]

use core::any::TypeId;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;

use sigil_actions::ActionRegistry;

use crate::arena::Arena;
use crate::async_cx::{MainThreadDispatcher, TaskExecutor};
use crate::focus::FocusHandleId;
use crate::local_executor::LocalTaskExecutor;
use crate::phase::{DeferredWrite, DrawPhase};
pub(crate) use crate::runtime_slots::{
    EffectKey, EffectSlot, EntityKey, EntitySlot, ObserveBareFn, ObserveFn, ObserverKey,
    ObserverKind, ObserverSlot, OwnerKey, OwnerSlot, PendingCompletion, PendingObserver, SignalKey,
    SignalSlot, SubscribeFn,
};

pub(crate) const MAX_QUIESCENCE_PASSES: u32 = 64;

static RUNTIME_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct RuntimeState {
    pub(crate) owners: Arena<OwnerSlot>,
    pub(crate) signals: Arena<SignalSlot>,
    pub(crate) effects: Arena<EffectSlot>,
    pub(crate) entities: Arena<EntitySlot>,
    pub(crate) observers: Arena<ObserverSlot>,
    pub(crate) pending_observers: VecDeque<PendingObserver>,
    pub(crate) update_depth: u32,
    pub(crate) globals: Vec<(TypeId, EntityKey)>,
    pub(crate) current_owner: Option<OwnerKey>,
    pub(crate) current_effect: Option<EffectKey>,
    pub(crate) current_entity: Option<EntityKey>,
    pub(crate) batch_depth: u32,
    pub(crate) pending: VecDeque<EffectKey>,
    pub(crate) pending_set: BTreeSet<EffectKey>,
    pub(crate) running: BTreeSet<EffectKey>,
    pub(crate) focus_stack: Vec<FocusHandleId>,
    pub(crate) action_registry: Option<ActionRegistry>,
    pub(crate) phase: DrawPhase,
    pub(crate) deferred_writes: Vec<DeferredWrite>,
    pub(crate) pending_completions: Vec<PendingCompletion>,
    pub(crate) executor: Option<Arc<dyn TaskExecutor>>,
    pub(crate) dispatcher: Option<Arc<dyn MainThreadDispatcher>>,
    pub(crate) local_executor: Option<Arc<dyn LocalTaskExecutor>>,
    pub(crate) frame_dirty: bool,
    pub(crate) runtime_id: u64,
}

impl RuntimeState {
    const fn new() -> Self {
        Self {
            owners: Arena::new(),
            signals: Arena::new(),
            effects: Arena::new(),
            entities: Arena::new(),
            observers: Arena::new(),
            pending_observers: VecDeque::new(),
            update_depth: 0,
            globals: Vec::new(),
            current_owner: None,
            current_effect: None,
            current_entity: None,
            batch_depth: 0,
            pending: VecDeque::new(),
            pending_set: BTreeSet::new(),
            running: BTreeSet::new(),
            focus_stack: Vec::new(),
            action_registry: None,
            phase: DrawPhase::Idle,
            deferred_writes: Vec::new(),
            pending_completions: Vec::new(),
            executor: None,
            dispatcher: None,
            local_executor: None,
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
        let owner = crate::owner::Owner::new(name, None);
        let result = owner.with(f);
        owner.dispose();
        result
    }

    #[must_use]
    pub fn current_phase() -> DrawPhase {
        with_rt(|rt| rt.phase)
    }
}
