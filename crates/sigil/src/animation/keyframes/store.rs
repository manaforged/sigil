use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use super::Keyframes;
use crate::reactive::{Owner, cleanup};

const CAPACITY: usize = 256;
const KEEP_RECENT: u64 = 128;

kittest_pub! {
    /// Names one keyframe animation in the shared store, so a style atom stays `Copy`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct KeyframesId {
        slot: u32,
        generation: u32,
    }
}

struct Slot {
    keyframes: Option<Arc<Keyframes>>,
    generation: u32,
    used: u64,
}

struct Store {
    index: HashMap<Arc<Keyframes>, u32>,
    slots: Vec<Slot>,
    free: Vec<u32>,
    clock: u64,
}

thread_local! {
    static PINS: RefCell<HashSet<(Owner, KeyframesId)>> = RefCell::new(HashSet::new());
    static MISSED: RefCell<HashSet<KeyframesId>> = RefCell::new(HashSet::new());
}

fn pin(id: KeyframesId, keyframes: Arc<Keyframes>) {
    let Some(owner) = Owner::current() else {
        return;
    };
    if !PINS.with_borrow_mut(|pins| pins.insert((owner, id))) {
        return;
    }
    cleanup(move || {
        drop(keyframes);
        PINS.with_borrow_mut(|pins| pins.remove(&(owner, id)));
    });
}

fn report_miss(id: KeyframesId) {
    if MISSED.with_borrow_mut(|missed| missed.insert(id)) {
        tracing::warn!(
            target: "sigil::animation",
            ?id,
            "a keyframe animation built outside any reactive scope was evicted before it played"
        );
    }
}

fn with_store<R>(job: impl FnOnce(&mut Store) -> R) -> R {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    let mut store = STORE
        .get_or_init(|| Mutex::new(Store::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    job(&mut store)
}

impl KeyframesId {
    pub(crate) fn new(keyframes: Keyframes) -> Self {
        let (id, keyframes) = with_store(|store| store.insert(keyframes));
        pin(id, keyframes);
        id
    }

    pub(crate) fn get(self) -> Option<Arc<Keyframes>> {
        let found = with_store(|store| {
            let slot = store.slots.get(self.slot as usize)?;
            (slot.generation == self.generation)
                .then(|| slot.keyframes.clone())
                .flatten()
        });
        if found.is_none() {
            report_miss(self);
        }
        found
    }
}

impl Store {
    fn new() -> Self {
        Self {
            index: HashMap::with_capacity(2 * CAPACITY),
            slots: Vec::with_capacity(CAPACITY),
            free: Vec::with_capacity(CAPACITY),
            clock: 0,
        }
    }

    fn insert(&mut self, keyframes: Keyframes) -> (KeyframesId, Arc<Keyframes>) {
        self.clock += 1;
        let clock = self.clock;
        if let Some(&slot) = self.index.get(&keyframes)
            && let Some(entry) = self.slots.get_mut(slot as usize)
            && let Some(held) = entry.keyframes.clone()
        {
            entry.used = clock;
            let id = KeyframesId {
                slot,
                generation: entry.generation,
            };
            return (id, held);
        }
        if self.index.len() >= CAPACITY {
            self.evict_one();
        }
        let keyframes = Arc::new(keyframes);
        let slot = self.claim();
        let generation = self.slots.get_mut(slot as usize).map_or(0, |entry| {
            entry.keyframes = Some(Arc::clone(&keyframes));
            entry.used = clock;
            entry.generation
        });
        self.index.insert(Arc::clone(&keyframes), slot);
        (KeyframesId { slot, generation }, keyframes)
    }

    fn claim(&mut self) -> u32 {
        self.free.pop().unwrap_or_else(|| {
            self.slots.push(Slot {
                keyframes: None,
                generation: 0,
                used: 0,
            });
            u32::try_from(self.slots.len() - 1).unwrap_or(u32::MAX)
        })
    }

    fn evict_one(&mut self) {
        let horizon = self.clock.saturating_sub(KEEP_RECENT);
        let idle = |slot: &Slot| {
            slot.used < horizon
                && slot
                    .keyframes
                    .as_ref()
                    .is_some_and(|keyframes| Arc::strong_count(keyframes) == 2)
        };
        let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .filter(|(_, slot)| idle(slot))
            .min_by_key(|(_, slot)| slot.used)
        else {
            return;
        };
        if let Some(keyframes) = slot.keyframes.take() {
            self.index.remove(&*keyframes);
        }
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(u32::try_from(index).unwrap_or(u32::MAX));
    }
}
