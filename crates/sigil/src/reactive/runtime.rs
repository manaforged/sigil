use core::cell::RefCell;
use core::panic::Location;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{BTreeSet, VecDeque};

use crate::reactive::arena::Arena;
use crate::reactive::phase::DrawPhase;
pub(crate) use crate::reactive::runtime_slots::{
    EffectKey, EffectSlot, OwnerKey, OwnerSlot, SignalKey, SignalSlot,
};

pub(crate) type AfterRun = (Option<EffectKey>, Box<dyn FnOnce()>);

static RUNTIME_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

const MAX_LEVEL: u32 = 1023;

pub(crate) struct RuntimeState {
    pub(crate) owners: Arena<OwnerSlot>,
    pub(crate) signals: Arena<SignalSlot>,
    pub(crate) effects: Arena<EffectSlot>,
    pub(crate) current_owner: Option<OwnerKey>,
    pub(crate) current_effect: Option<EffectKey>,
    pub(crate) batch_depth: u32,
    pending: Vec<VecDeque<(u64, EffectKey)>>,
    pending_floor: usize,
    pending_seq: u64,
    pub(crate) pending_len: usize,
    pub(crate) running: BTreeSet<EffectKey>,
    pub(crate) dispose_after_run: Vec<EffectKey>,
    pub(crate) deferred_cleanups: Vec<Box<dyn FnOnce()>>,
    pub(crate) after_run: Vec<AfterRun>,
    pub(crate) panic_origin: Option<(EffectKey, &'static str, &'static Location<'static>)>,
    pub(crate) phase: DrawPhase,
    pub(crate) frame_dirty: bool,
    pub(crate) paints: u64,
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
            pending: Vec::new(),
            pending_floor: 0,
            pending_seq: 0,
            pending_len: 0,
            running: BTreeSet::new(),
            dispose_after_run: Vec::new(),
            deferred_cleanups: Vec::new(),
            after_run: Vec::new(),
            panic_origin: None,
            phase: DrawPhase::Idle,
            frame_dirty: true,
            paints: 0,
            runtime_id: 0,
        }
    }

    pub(crate) fn enqueue(&mut self, effect: EffectKey, floor: u32) {
        let Some(slot) = self.effects.get_mut(effect) else {
            return;
        };
        if slot.queued.is_some() {
            return;
        }
        let level = slot.level.max(floor) as usize;
        slot.queued = Some(self.pending_seq);
        if self.pending.len() <= level {
            self.pending.resize_with(level + 1, VecDeque::new);
        }
        if let Some(queue) = self.pending.get_mut(level) {
            queue.push_back((self.pending_seq, effect));
        }
        self.pending_seq += 1;
        self.pending_len += 1;
        self.pending_floor = self.pending_floor.min(level);
    }

    pub(crate) fn dequeue(&mut self) -> Option<EffectKey> {
        while let Some(queue) = self.pending.get_mut(self.pending_floor) {
            let Some((seq, effect)) = queue.pop_front() else {
                self.pending_floor += 1;
                continue;
            };
            let Some(slot) = self.effects.get_mut(effect) else {
                continue;
            };
            if slot.queued == Some(seq) {
                slot.queued = None;
                self.pending_len -= 1;
                return Some(effect);
            }
        }
        None
    }

    pub(crate) fn writer_level(&self, written: SignalKey) -> u32 {
        self.current_effect
            .and_then(|effect| self.effects.get(effect))
            .filter(|slot| !slot.deps.contains(&written) && !slot.prev_deps.contains(&written))
            .map_or(0, |slot| (slot.level + 1).min(MAX_LEVEL))
    }

    pub(crate) fn creator_level(&self) -> u32 {
        self.current_effect
            .and_then(|effect| self.effects.get(effect))
            .map_or(0, |slot| (slot.level + 1).min(MAX_LEVEL))
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

kittest_pub! {
    /// Run code inside a reactive runtime.
    #[derive(Debug)]
    pub struct Runtime;
}

impl Runtime {
    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn current_phase() -> DrawPhase {
        with_rt(|rt| rt.phase)
    }
}
