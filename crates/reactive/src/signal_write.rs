//! Signal write path — `set`, `update`, fan-out helpers, and guard.
//!
//! Separated from struct definitions and the read path to keep each
//! file under the 400-line cap. Not re-exported.

use core::panic::Location;

use smallvec::SmallVec;

use crate::effect::schedule_effects;
use crate::runtime::{EffectKey, EffectSlot, SignalKey, with_rt};
use crate::signal::Signal;

/// Sentinel placed in a signal's slot while a `Signal::update`
/// closure is running. Lets `get`/`peek`/`set`/`update` detect
/// re-entry on the same signal and produce an actionable panic
/// rather than a confusing "type mismatch" downcast failure.
pub(crate) struct UpdateInProgress;

fn is_stale_running_subscription(
    effects: &crate::arena::Arena<EffectSlot>,
    running: &std::collections::BTreeSet<EffectKey>,
    effect: EffectKey,
    signal: SignalKey,
) -> bool {
    running.contains(&effect)
        && effects
            .get(effect)
            .is_some_and(|slot| slot.prev_deps.contains(&signal) && !slot.deps.contains(&signal))
}

/// Runtime fields needed for fanning out a signal write.
/// Groups 5 borrows so `fan_out_signal` stays within the 5-arg limit.
struct SignalFanOut<'a> {
    batch_depth: u32,
    effects: &'a crate::arena::Arena<EffectSlot>,
    running: &'a std::collections::BTreeSet<EffectKey>,
    pending: &'a mut std::collections::VecDeque<EffectKey>,
    pending_set: &'a mut std::collections::BTreeSet<EffectKey>,
}

/// Fan out a signal write to subscribed effects.
///
/// Batch path: enqueues directly into the pending queue and returns
/// `None`. Non-batch path: returns the live set so the caller can
/// run effects synchronously after releasing the runtime borrow.
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

/// Replace a signal slot with the `UpdateInProgress` sentinel and
/// return the original payload, asserting paint-safety + non-disposed +
/// non-reentrant access. Lifted out of [`Signal::update`] to keep that
/// method within the 50-line cap.
#[track_caller]
fn take_for_update(key: SignalKey, caller: &'static Location<'static>) -> Box<dyn core::any::Any> {
    with_rt(|rt| {
        crate::phase::assert_paint_safe(rt, "Signal::update");
        let Some(slot) = rt.signals.get_mut(key) else {
            crate::diagnostic::disposed_signal("update", caller);
        };
        assert!(
            !slot.value.is::<UpdateInProgress>(),
            "Signal::update called recursively on the same signal (at {caller})"
        );
        core::mem::replace(&mut slot.value, Box::new(UpdateInProgress))
    })
}

impl<T: 'static> Signal<T> {
    /// Replace the signal's value and schedule dependent effects.
    ///
    /// The old `T` is dropped *outside* the runtime borrow so a
    /// `Drop` impl that touches the runtime cannot deadlock.
    ///
    /// # Panics
    ///
    /// Panics if the signal has been disposed.
    #[inline]
    #[track_caller]
    pub fn set(self, value: T) {
        let caller = Location::caller();
        let (old, subscribers) = with_rt(|rt| {
            crate::phase::assert_paint_safe(rt, "Signal::set");
            let Some(slot) = rt.signals.get_mut(self.key) else {
                crate::diagnostic::disposed_signal("set", caller);
            };
            assert!(
                !slot.value.is::<UpdateInProgress>(),
                "Signal::set on a signal whose own update closure is still running (at {caller})"
            );
            let current = slot
                .value
                .downcast_mut::<T>()
                .expect("signal type mismatch");
            let old = core::mem::replace(current, value);
            if slot.subscribers.is_empty() {
                return (old, None);
            }
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

    /// Mutate the signal's value in place and schedule dependent effects.
    ///
    /// `f` runs *outside* the runtime borrow, so it may freely call
    /// `get`/`set`/`update` on *other* signals. Re-entering the same
    /// signal panics — the slot is locked while `f` runs.
    ///
    /// If `f` panics the signal's value is restored via an RAII guard
    /// and the `UpdateInProgress` sentinel is cleared.
    ///
    /// # Panics
    ///
    /// Panics if the signal has been disposed, or if `f` calls back
    /// into this same signal.
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

/// RAII guard that restores a signal's slot value on drop — used by
/// `Signal::update` so a panic inside the user closure still clears
/// the `UpdateInProgress` sentinel and leaves the signal readable.
///
/// Normal-return code path `take()`s the value out before the guard
/// drops, so `Drop` only runs the restore on an unwind.
struct UpdateGuard {
    key: SignalKey,
    value: Option<Box<dyn core::any::Any>>,
}

impl Drop for UpdateGuard {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            with_rt(|rt| {
                if let Some(slot) = rt.signals.get_mut(self.key) {
                    slot.value = value;
                }
                // If the slot is gone the value drops here.
            });
        }
    }
}
