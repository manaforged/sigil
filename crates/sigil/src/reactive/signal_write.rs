use core::any::Any;
use core::panic::Location;

use smallvec::SmallVec;

use crate::reactive::batch::settle;
use crate::reactive::diagnostic::disposed_signal;
use crate::reactive::phase::assert_paint_safe;
use crate::reactive::runtime::{EffectKey, RuntimeState, SignalKey, SignalSlot, with_rt};
use crate::reactive::signal::Signal;

pub(crate) struct UpdateInProgress;

pub(crate) fn live_slot<'a>(
    rt: &'a mut RuntimeState,
    key: SignalKey,
    op: &'static str,
    caller: &'static Location<'static>,
) -> &'a mut SignalSlot {
    let Some(slot) = rt.signals.get_mut(key) else {
        disposed_signal(op, caller);
    };
    assert!(
        !slot.value.is::<UpdateInProgress>(),
        "Signal::{op} on a signal whose own update or with closure is still running (at {caller})"
    );
    slot
}

pub(crate) fn take_value(
    rt: &mut RuntimeState,
    key: SignalKey,
    op: &'static str,
    caller: &'static Location<'static>,
) -> Box<dyn Any> {
    core::mem::replace(
        &mut live_slot(rt, key, op, caller).value,
        Box::new(UpdateInProgress),
    )
}

fn is_stale_running_subscription(
    effects: &crate::reactive::arena::Arena<crate::reactive::runtime_slots::EffectSlot>,
    running: &std::collections::BTreeSet<EffectKey>,
    effect: EffectKey,
    signal: SignalKey,
) -> bool {
    running.contains(&effect)
        && effects
            .get(effect)
            .is_some_and(|slot| slot.prev_deps.contains(&signal) && !slot.deps.contains(&signal))
}

fn restamp(rt: &mut RuntimeState, signal: SignalKey) {
    let stamp = rt.next_stamp();
    if let Some(slot) = rt.signals.get_mut(signal) {
        slot.stamp = stamp;
    }
}

fn notify(rt: &mut RuntimeState, signal: SignalKey) -> bool {
    let written = rt.writer_level(signal);
    let Some(slot) = rt.signals.get_mut(signal) else {
        return false;
    };
    slot.level = slot.level.max(written);
    let level = slot.level;
    if slot.subscribers.is_empty() {
        return false;
    }
    let (effects, running) = (&rt.effects, &rt.running);
    let live: SmallVec<[EffectKey; 4]> = slot
        .subscribers
        .iter()
        .copied()
        .filter(|&e| !is_stale_running_subscription(effects, running, e, signal))
        .collect();
    rt.ask_frame();
    let queued = !live.is_empty();
    for effect in live {
        rt.enqueue(effect, level);
    }
    queued
}

impl<T: PartialEq + 'static> Signal<T> {
    /// Writes `value` and notifies subscribers.
    #[inline]
    #[track_caller]
    pub fn set(self, value: T) {
        self.write("set", Location::caller(), value, T::eq);
    }

    #[track_caller]
    pub(crate) fn try_set(self, value: T) -> bool {
        self.when_writable(|| self.set(value))
    }
}

impl<T: 'static> Signal<T> {
    #[inline]
    #[track_caller]
    fn write(
        self,
        op: &'static str,
        caller: &'static Location<'static>,
        value: T,
        unchanged: fn(&T, &T) -> bool,
    ) {
        let (old, queued) = with_rt(|rt| {
            assert_paint_safe(rt, op);
            let slot = live_slot(rt, self.key, op, caller);
            let current = slot
                .value
                .downcast_mut::<T>()
                .expect("signal type mismatch");
            if unchanged(current, &value) {
                return (None, false);
            }
            let old = core::mem::replace(current, value);
            restamp(rt, self.key);
            (Some(old), notify(rt, self.key))
        });
        drop(old);
        if queued {
            settle();
        }
    }

    #[inline]
    #[track_caller]
    pub(crate) fn set_always(self, value: T) {
        self.write("set_always", Location::caller(), value, |_, _| false);
    }

    #[track_caller]
    pub(crate) fn try_set_always(self, value: T) -> bool {
        self.when_writable(|| self.set_always(value))
    }

    pub(crate) fn try_update(self, f: impl FnOnce(&mut T)) -> bool {
        self.when_writable(|| self.update(f))
    }

    #[track_caller]
    fn when_writable(self, write: impl FnOnce()) -> bool {
        let writable = self.is_writable();
        if writable {
            write();
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

    /// Changes the value in place and notifies subscribers.
    #[track_caller]
    pub fn update(self, f: impl FnOnce(&mut T)) {
        let caller = Location::caller();
        let taken = with_rt(|rt| {
            assert_paint_safe(rt, "update");
            let taken = take_value(rt, self.key, "update", caller);
            restamp(rt, self.key);
            taken
        });
        let mut guard = UpdateGuard::new(self.key, taken);
        f(guard
            .value
            .as_mut()
            .and_then(|value| value.downcast_mut::<T>())
            .expect("signal type mismatch"));
        let restored = guard.value.take();
        let queued = with_rt(|rt| {
            let (Some(slot), Some(value)) = (rt.signals.get_mut(self.key), restored) else {
                return false;
            };
            slot.value = value;
            notify(rt, self.key)
        });
        if queued {
            settle();
        }
    }
}

pub(crate) struct UpdateGuard {
    key: SignalKey,
    value: Option<Box<dyn Any>>,
}

impl UpdateGuard {
    pub(crate) const fn new(key: SignalKey, value: Box<dyn Any>) -> Self {
        Self {
            key,
            value: Some(value),
        }
    }

    pub(crate) fn value_ref<T: 'static>(&self) -> &T {
        self.value
            .as_ref()
            .and_then(|value| value.downcast_ref::<T>())
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
