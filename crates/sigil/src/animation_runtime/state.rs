use core::cell::RefCell;
use slotmap::{SlotMap, new_key_type};

use crate::animation::{Easing, SpringPreset};
use crate::reactive::{Runtime, Signal, batch};

use crate::animation_runtime::MotionPrefs;
use crate::animation_runtime::animator::{SpringMotion, TweenMotion};
use crate::animation_runtime::motion_state::Motion;

new_key_type! {
    pub(crate) struct AnimatorId;
}

struct Entry {
    signal: Signal<f32>,
    motion: Motion,
    paused: bool,
}

thread_local! {
    static ENTRIES: RefCell<SlotMap<AnimatorId, Entry>> = RefCell::new(SlotMap::with_key());
}

pub(crate) fn register_spring(signal: Signal<f32>, preset: SpringPreset) -> AnimatorId {
    let motion = Motion::Spring(SpringMotion::from_preset(
        signal.try_peek().unwrap_or_default(),
        preset,
    ));
    register(signal, motion)
}

pub(crate) fn register_tween(signal: Signal<f32>, ms: u16, easing: Easing) -> AnimatorId {
    let motion = Motion::Tween(TweenMotion::new(
        signal.try_peek().unwrap_or_default(),
        ms,
        easing,
    ));
    register(signal, motion)
}

pub(crate) fn register(signal: Signal<f32>, motion: Motion) -> AnimatorId {
    assert!(
        signal.try_peek().is_none_or(f32::is_finite),
        "animation value must be finite"
    );
    ENTRIES.with(|cell| {
        cell.borrow_mut().insert(Entry {
            signal,
            motion,
            paused: false,
        })
    })
}

impl AnimatorId {
    pub(crate) fn pause(self, paused: bool) {
        self.update(|entry| entry.paused = paused);
    }

    pub(crate) fn finish(self) {
        self.update(|entry| entry.motion.finish());
    }

    pub(crate) fn seek(self, seconds: f64) {
        self.update(|entry| {
            if let Motion::Keyframes(motion) = &mut entry.motion {
                motion.seek(seconds);
            }
        });
    }

    pub(crate) fn cancel(self) {
        self.update(|entry| {
            if let Motion::Keyframes(motion) = &mut entry.motion {
                motion.cancel();
            }
        });
    }

    fn update(self, change: impl FnOnce(&mut Entry)) {
        batch(|| {
            ENTRIES.with(|cell| {
                let mut entries = cell.borrow_mut();
                let Some(entry) = entries.get_mut(self) else {
                    return;
                };
                change(entry);
                if MotionPrefs::reduced() && !entry.motion.is_settled() {
                    entry.motion.finish();
                }
                if !entry.signal.try_set(entry.motion.value()) {
                    entries.remove(self);
                }
                Runtime::mark_frame_dirty();
            });
        });
    }

    pub(crate) fn deregister(self) {
        ENTRIES.with(|cell| {
            cell.borrow_mut().remove(self);
        });
    }

    pub(crate) fn retarget(self, value: f32) {
        batch(|| {
            ENTRIES.with(|cell| {
                let mut entries = cell.borrow_mut();
                if let Some(entry) = entries.get_mut(self) {
                    entry.motion.retarget(value);
                    entry.paused = false;
                    Runtime::mark_frame_dirty();
                    if entry.motion.is_settled() && !entry.signal.try_set(entry.motion.value()) {
                        entries.remove(self);
                    }
                }
            });
        });
    }

    pub(crate) fn is_settled(self) -> bool {
        ENTRIES.with(|cell| {
            let entries = cell.borrow();
            entries
                .get(self)
                .is_none_or(|entry| entry.motion.is_settled())
        })
    }

    pub(crate) fn peek_value(self) -> Option<f32> {
        ENTRIES.with(|cell| cell.borrow().get(self).map(|entry| entry.motion.value()))
    }
}

pub(crate) fn finish_all() {
    batch(|| {
        ENTRIES.with(|cell| {
            cell.borrow_mut().retain(|_, entry| {
                entry.motion.finish();
                entry.signal.try_set(entry.motion.value())
            });
            Runtime::mark_frame_dirty();
        });
    });
}

pub fn tick(dt: f32) {
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }
    batch(|| {
        ENTRIES.with(|cell| {
            cell.borrow_mut().retain(|_, entry| {
                if entry.paused || entry.motion.is_settled() {
                    return true;
                }
                let _ = entry.motion.tick(dt);
                Runtime::mark_frame_dirty();
                entry.signal.try_set(entry.motion.value())
            });
        });
    });
}

#[must_use]
pub fn active_count() -> usize {
    ENTRIES.with(|cell| {
        cell.borrow()
            .values()
            .filter(|entry| !entry.paused && !entry.motion.is_settled())
            .count()
    })
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
