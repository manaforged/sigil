use core::cell::{Cell, RefCell};

use crate::animation::{Easing, SpringPreset};
use crate::reactive::Signal;

use crate::animation_runtime::animator::{Motion, SpringMotion, TweenMotion};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnimatorId(u32);

struct Entry {
    id: AnimatorId,
    signal: Signal<f32>,
    motion: Box<dyn Motion>,
}

thread_local! {
    static ENTRIES: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
    static NEXT_ID: Cell<u32> = const { Cell::new(1) };
}

fn next_id() -> AnimatorId {
    NEXT_ID.with(|cell| {
        let id = cell.get();
        cell.set(id.wrapping_add(1).max(1));
        AnimatorId(id)
    })
}

pub(crate) fn register_spring(signal: Signal<f32>, preset: SpringPreset) -> AnimatorId {
    let id = next_id();
    let motion = Box::new(SpringMotion::from_preset(signal.peek(), preset));
    ENTRIES.with(|cell| {
        cell.borrow_mut().push(Entry { id, signal, motion });
    });
    id
}

pub(crate) fn register_tween(signal: Signal<f32>, ms: u16, easing: Easing) -> AnimatorId {
    let id = next_id();
    let motion = Box::new(TweenMotion::new(signal.peek(), ms, easing));
    ENTRIES.with(|cell| {
        cell.borrow_mut().push(Entry { id, signal, motion });
    });
    id
}

pub(crate) fn deregister(id: AnimatorId) {
    ENTRIES.with(|cell| {
        let mut entries = cell.borrow_mut();
        if let Some(pos) = entries.iter().position(|entry| entry.id == id) {
            entries.swap_remove(pos);
        }
    });
}

pub(crate) fn retarget(id: AnimatorId, value: f32) {
    ENTRIES.with(|cell| {
        let mut entries = cell.borrow_mut();
        if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
            entry.motion.retarget(value);
        }
    });
}

pub(crate) fn is_settled(id: AnimatorId) -> bool {
    ENTRIES.with(|cell| {
        let entries = cell.borrow();
        entries
            .iter()
            .find(|entry| entry.id == id)
            .is_none_or(|entry| entry.motion.is_settled())
    })
}

pub(crate) fn peek_value(id: AnimatorId) -> Option<f32> {
    ENTRIES.with(|cell| {
        cell.borrow()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.motion.value())
    })
}

pub fn tick(dt: f32) {
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }
    let drained: Vec<Entry> = ENTRIES.with(|cell| core::mem::take(&mut *cell.borrow_mut()));
    let mut kept: Vec<Entry> = Vec::with_capacity(drained.len());
    for mut entry in drained {
        if !entry.motion.is_settled() {
            let _ = entry.motion.tick(dt);
            entry.signal.set(entry.motion.value());
        }
        kept.push(entry);
    }
    ENTRIES.with(|cell| {
        let mut entries = cell.borrow_mut();
        if entries.is_empty() {
            *entries = kept;
        } else {
            entries.splice(0..0, kept);
        }
    });
}

#[must_use]
pub fn active_count() -> usize {
    ENTRIES.with(|cell| {
        cell.borrow()
            .iter()
            .filter(|entry| !entry.motion.is_settled())
            .count()
    })
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
