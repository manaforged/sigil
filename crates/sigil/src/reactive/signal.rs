use core::marker::PhantomData;
use core::panic::Location;

use smallvec::SmallVec;

use crate::reactive::runtime::{
    EffectKey, RuntimeState, SignalKey, SignalSlot, runtime_id, with_rt,
};
use crate::reactive::signal_write::{UpdateGuard, UpdateInProgress};

pub struct Signal<T: 'static> {
    pub(crate) key: SignalKey,
    _not_send: PhantomData<*const T>,
}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Signal<T> {}

impl<T: 'static> Signal<T> {
    pub(crate) const fn from_key(key: SignalKey) -> Self {
        Self {
            key,
            _not_send: PhantomData,
        }
    }

    #[must_use]
    pub fn into_sendable(self) -> SendableSignal<T> {
        SendableSignal {
            index: self.key.index_for_sendable(),
            generation: self.key.generation_for_sendable(),
            runtime_id: runtime_id(),
            _ty: PhantomData,
        }
    }
}

pub struct SendableSignal<T: 'static> {
    index: u32,
    generation: u32,
    runtime_id: u64,
    _ty: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for SendableSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for SendableSignal<T> {}

impl<T: 'static> core::fmt::Debug for SendableSignal<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "SendableSignal<{}>({}@{} rt#{})",
            core::any::type_name::<T>(),
            self.index,
            self.generation,
            self.runtime_id,
        )
    }
}

impl<T: 'static> SendableSignal<T> {
    #[must_use]
    #[track_caller]
    pub fn into_signal(self) -> Signal<T> {
        let current = runtime_id();
        assert!(
            current == self.runtime_id,
            "SendableSignal rehydrated on wrong runtime:\n  \
             signal created in runtime #{original}\n  \
             rehydrated in runtime #{current}\n  \
             signals do not cross runtime boundaries",
            original = self.runtime_id,
            current = current,
        );
        Signal::from_key(SignalKey::from_sendable(self.index, self.generation))
    }
}

impl<T: 'static> core::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Signal<{}>({:?})", core::any::type_name::<T>(), self.key)
    }
}

#[must_use]
#[track_caller]
pub fn signal<T: 'static>(initial: T) -> Signal<T> {
    let location = Location::caller();
    let key = with_rt(|rt| {
        let owner = rt
            .current_owner
            .unwrap_or_else(|| crate::reactive::diagnostic::no_scope("signal()", location));
        assert!(
            rt.owners.contains(owner),
            "signal() with a disposed current owner (id={owner:?}, at {location})",
        );
        let key = rt.signals.insert(SignalSlot {
            owner,
            value: Box::new(initial),
            subscribers: SmallVec::new(),
            location,
        });
        rt.owners
            .get_mut(owner)
            .expect("current owner slot missing")
            .signals
            .push(key);
        key
    });
    Signal {
        key,
        _not_send: PhantomData,
    }
}

impl<T: Clone + 'static> Signal<T> {
    #[inline]
    #[track_caller]
    #[must_use]
    pub fn get(self) -> T {
        let caller = Location::caller();
        with_rt(|rt| {
            let Some(slot) = rt.signals.get(self.key) else {
                crate::reactive::diagnostic::disposed_signal("get", caller);
            };
            assert!(
                !slot
                    .value
                    .is::<crate::reactive::signal_write::UpdateInProgress>(),
                "Signal::get on a signal whose own update closure is still running (at {caller})"
            );
            let value = slot
                .value
                .downcast_ref::<T>()
                .expect("signal type mismatch")
                .clone();
            track_key(rt, self.key);
            value
        })
    }

    #[inline]
    #[track_caller]
    #[must_use]
    pub fn peek(self) -> T {
        let caller = Location::caller();
        with_rt(|rt| {
            let Some(slot) = rt.signals.get(self.key) else {
                crate::reactive::diagnostic::disposed_signal("peek", caller);
            };
            assert!(
                !slot
                    .value
                    .is::<crate::reactive::signal_write::UpdateInProgress>(),
                "Signal::peek on a signal whose own update closure is still running (at {caller})"
            );
            slot.value
                .downcast_ref::<T>()
                .expect("signal type mismatch")
                .clone()
        })
    }

    #[inline]
    #[must_use]
    pub fn try_peek(self) -> Option<T> {
        with_rt(|rt| {
            let slot = rt.signals.get(self.key)?;
            if slot
                .value
                .is::<crate::reactive::signal_write::UpdateInProgress>()
            {
                return None;
            }
            slot.value.downcast_ref::<T>().cloned()
        })
    }
}

fn track_key(rt: &mut RuntimeState, key: SignalKey) {
    if let Some(effect) = rt.current_effect {
        let should_subscribe = if let Some(eff) = rt.effects.get_mut(effect) {
            if eff
                .prev_deps
                .get(eff.prev_dep_cursor)
                .is_some_and(|&dep| dep == key)
            {
                eff.prev_dep_cursor += 1;
                eff.deps.push(key);
                false
            } else if eff.deps.last().is_some_and(|&dep| dep == key) || eff.deps.contains(&key) {
                false
            } else {
                let already_subscribed = eff.prev_deps.contains(&key);
                eff.deps.push(key);
                !already_subscribed
            }
        } else {
            false
        };
        if should_subscribe && let Some(sub_slot) = rt.signals.get_mut(key) {
            sub_slot.subscribers.push(effect);
        }
    }
}

impl<T: 'static> Signal<T> {
    #[track_caller]
    pub fn track(self) {
        let caller = Location::caller();
        with_rt(|rt| {
            if rt.signals.get(self.key).is_none() {
                crate::reactive::diagnostic::disposed_signal("track", caller);
            }
            track_key(rt, self.key);
        });
    }

    #[track_caller]
    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let caller = Location::caller();
        let taken = with_rt(|rt| {
            let Some(slot) = rt.signals.get_mut(self.key) else {
                crate::reactive::diagnostic::disposed_signal("with", caller);
            };
            assert!(
                !slot.value.is::<UpdateInProgress>(),
                "Signal::with on a signal whose own update or with closure is still running (at {caller})"
            );
            let taken = core::mem::replace(&mut slot.value, Box::new(UpdateInProgress));
            track_key(rt, self.key);
            taken
        });
        let guard = UpdateGuard::new(self.key, taken);
        let out = f(guard.value_ref::<T>());
        drop(guard);
        out
    }
}

const _: () = {
    let _ = core::mem::size_of::<EffectKey>();
};
