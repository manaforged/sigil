use core::panic::Location;

use smallvec::SmallVec;

use crate::reactive::effect::schedule_effects;
use crate::reactive::runtime::{EffectKey, EffectSlot, SignalKey, with_rt};
use crate::reactive::signal::Signal;

pub(crate) struct UpdateInProgress;

fn is_stale_running_subscription(
    effects: &crate::reactive::arena::Arena<EffectSlot>,
    running: &std::collections::BTreeSet<EffectKey>,
    effect: EffectKey,
    signal: SignalKey,
) -> bool {
    running.contains(&effect)
        && effects
            .get(effect)
            .is_some_and(|slot| slot.prev_deps.contains(&signal) && !slot.deps.contains(&signal))
}

struct SignalFanOut<'a> {
    batch_depth: u32,
    effects: &'a crate::reactive::arena::Arena<EffectSlot>,
    running: &'a std::collections::BTreeSet<EffectKey>,
    pending: &'a mut std::collections::VecDeque<EffectKey>,
    pending_set: &'a mut std::collections::BTreeSet<EffectKey>,
}

fn fan_out_signal(
    ctx: SignalFanOut<'_>,
    subscribers: &SmallVec<[EffectKey; 4]>,
    signal: SignalKey,
) -> Option<SmallVec<[EffectKey; 4]>> {
    let SignalFanOut {
        batch_depth,
        effects,
        running,
        pending,
        pending_set,
    } = ctx;
    if batch_depth > 0 {
        if running.is_empty() {
            for &effect in subscribers {
                if pending_set.insert(effect) {
                    pending.push_back(effect);
                }
            }
        } else {
            for &effect in subscribers {
                if is_stale_running_subscription(effects, running, effect, signal) {
                    continue;
                }
                if pending_set.insert(effect) {
                    pending.push_back(effect);
                }
            }
        }
        None
    } else if running.is_empty() {
        Some(subscribers.clone())
    } else {
        let subs: SmallVec<[EffectKey; 4]> = subscribers
            .iter()
            .copied()
            .filter(|&e| !is_stale_running_subscription(effects, running, e, signal))
            .collect();
        if subs.is_empty() { None } else { Some(subs) }
    }
}

#[track_caller]
fn take_for_update(key: SignalKey, caller: &'static Location<'static>) -> Box<dyn core::any::Any> {
    with_rt(|rt| {
        crate::reactive::phase::assert_paint_safe(rt, "Signal::update");
        let Some(slot) = rt.signals.get_mut(key) else {
            crate::reactive::diagnostic::disposed_signal("update", caller);
        };
        assert!(
            !slot.value.is::<UpdateInProgress>(),
            "Signal::update called recursively on the same signal (at {caller})"
        );
        core::mem::replace(&mut slot.value, Box::new(UpdateInProgress))
    })
}

impl<T: PartialEq + 'static> Signal<T> {
    #[inline]
    #[track_caller]
    pub fn set(self, value: T) {
        let caller = Location::caller();
        let (old, subscribers) = with_rt(|rt| {
            crate::reactive::phase::assert_paint_safe(rt, "Signal::set");
            let Some(slot) = rt.signals.get_mut(self.key) else {
                crate::reactive::diagnostic::disposed_signal("set", caller);
            };
            assert!(
                !slot.value.is::<UpdateInProgress>(),
                "Signal::set on a signal whose own update closure is still running (at {caller})"
            );
            let current = slot
                .value
                .downcast_mut::<T>()
                .expect("signal type mismatch");
            if *current == value {
                return (None, None);
            }
            let old = core::mem::replace(current, value);
            if slot.subscribers.is_empty() {
                return (Some(old), None);
            }
            rt.frame_dirty = true;
            let subs = fan_out_signal(
                SignalFanOut {
                    batch_depth: rt.batch_depth,
                    effects: &rt.effects,
                    running: &rt.running,
                    pending: &mut rt.pending,
                    pending_set: &mut rt.pending_set,
                },
                &slot.subscribers,
                self.key,
            );
            (Some(old), subs)
        });
        drop(old);
        if let Some(subs) = subscribers {
            schedule_effects(subs);
        }
    }
}

impl<T: PartialEq + 'static> Signal<T> {
    #[track_caller]
    pub fn try_set(self, value: T) -> bool {
        let writable = self.is_writable();
        if writable {
            self.set(value);
        }
        writable
    }
}

impl<T: 'static> Signal<T> {
    #[track_caller]
    pub fn try_set_always(self, value: T) -> bool {
        let writable = self.is_writable();
        if writable {
            self.set_always(value);
        }
        writable
    }

    fn is_writable(self) -> bool {
        with_rt(|rt| {
            rt.signals
                .get(self.key)
                .is_some_and(|slot| !slot.value.is::<UpdateInProgress>() && slot.value.is::<T>())
        })
    }
}

impl<T: 'static> Signal<T> {
    #[inline]
    #[track_caller]
    pub fn set_always(self, value: T) {
        let caller = Location::caller();
        let (old, subscribers) = with_rt(|rt| {
            crate::reactive::phase::assert_paint_safe(rt, "Signal::set_always");
            let Some(slot) = rt.signals.get_mut(self.key) else {
                crate::reactive::diagnostic::disposed_signal("set_always", caller);
            };
            assert!(
                !slot.value.is::<UpdateInProgress>(),
                "Signal::set_always on a signal whose own update closure is still running (at {caller})"
            );
            let current = slot
                .value
                .downcast_mut::<T>()
                .expect("signal type mismatch");
            let old = core::mem::replace(current, value);
            if slot.subscribers.is_empty() {
                return (old, None);
            }
            rt.frame_dirty = true;
            let subs = fan_out_signal(
                SignalFanOut {
                    batch_depth: rt.batch_depth,
                    effects: &rt.effects,
                    running: &rt.running,
                    pending: &mut rt.pending,
                    pending_set: &mut rt.pending_set,
                },
                &slot.subscribers,
                self.key,
            );
            (old, subs)
        });
        drop(old);
        if let Some(subs) = subscribers {
            schedule_effects(subs);
        }
    }

    #[track_caller]
    pub fn update(self, f: impl FnOnce(&mut T)) {
        let caller = Location::caller();
        let taken = take_for_update(self.key, caller);
        let mut guard = UpdateGuard {
            key: self.key,
            value: Some(taken),
        };
        {
            let inner = guard
                .value
                .as_mut()
                .expect("value populated by guard ctor")
                .downcast_mut::<T>()
                .expect("signal type mismatch (slot inner)");
            f(inner);
        }
        let restored = guard
            .value
            .take()
            .expect("value present after normal return");
        let subscribers = with_rt(|rt| {
            let slot = rt.signals.get_mut(self.key)?;
            slot.value = restored;
            if slot.subscribers.is_empty() {
                return None;
            }
            rt.frame_dirty = true;
            fan_out_signal(
                SignalFanOut {
                    batch_depth: rt.batch_depth,
                    effects: &rt.effects,
                    running: &rt.running,
                    pending: &mut rt.pending,
                    pending_set: &mut rt.pending_set,
                },
                &slot.subscribers,
                self.key,
            )
        });
        drop(guard);
        if let Some(subs) = subscribers {
            schedule_effects(subs);
        }
    }
}

pub(crate) struct UpdateGuard {
    key: SignalKey,
    value: Option<Box<dyn core::any::Any>>,
}

impl UpdateGuard {
    pub(crate) const fn new(key: SignalKey, value: Box<dyn core::any::Any>) -> Self {
        Self {
            key,
            value: Some(value),
        }
    }

    pub(crate) fn value_ref<T: 'static>(&self) -> &T {
        self.value
            .as_ref()
            .expect("value populated by guard ctor")
            .downcast_ref::<T>()
            .expect("signal type mismatch")
    }
}

impl Drop for UpdateGuard {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            with_rt(|rt| {
                if let Some(slot) = rt.signals.get_mut(self.key) {
                    slot.value = value;
                }
            });
        }
    }
}
