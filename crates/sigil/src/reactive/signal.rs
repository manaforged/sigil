use core::marker::PhantomData;
use core::panic::Location;

use smallvec::SmallVec;

use crate::reactive::diagnostic::{disposed_signal, no_scope};
use crate::reactive::runtime::{
    OwnerKey, RuntimeState, SignalKey, SignalSlot, runtime_id, with_rt,
};
use crate::reactive::signal_write::{UpdateGuard, live_slot, take_value};

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

impl<T: 'static> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl<T: 'static> Eq for Signal<T> {}

impl<T: 'static> core::hash::Hash for Signal<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T: 'static> Signal<T> {
    pub(crate) const fn from_key(key: SignalKey) -> Self {
        Self {
            key,
            _not_send: PhantomData,
        }
    }

    #[must_use]
    pub(crate) fn into_sendable(self) -> SendableSignal<T> {
        SendableSignal {
            index: self.key.index_for_sendable(),
            generation: self.key.generation_for_sendable(),
            runtime_id: runtime_id(),
            _ty: PhantomData,
        }
    }
}

pub(crate) struct SendableSignal<T: 'static> {
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
    pub(crate) fn into_signal(self) -> Signal<T> {
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
    let owner = with_rt(|rt| rt.current_owner).unwrap_or_else(|| no_scope("signal()", location));
    signal_in(initial, owner, location)
}

pub(crate) fn signal_in<T: 'static>(
    initial: T,
    owner: OwnerKey,
    location: &'static Location<'static>,
) -> Signal<T> {
    let key = with_rt(|rt| {
        assert!(
            rt.owners.contains(owner),
            "signal created under a disposed owner (id={owner:?}, at {location})",
        );
        let key = rt.signals.insert(SignalSlot {
            value: Box::new(initial),
            subscribers: SmallVec::new(),
        });
        rt.owners
            .get_mut(owner)
            .expect("owner checked above")
            .signals
            .push(key);
        key
    });
    Signal::from_key(key)
}

impl<T: Clone + 'static> Signal<T> {
    #[inline]
    #[track_caller]
    fn read(self, op: &'static str, track: bool) -> T {
        let caller = Location::caller();
        with_rt(|rt| {
            let value = live_slot(rt, self.key, op, caller)
                .value
                .downcast_ref::<T>()
                .expect("signal type mismatch")
                .clone();
            if track {
                track_key(rt, self.key);
            }
            value
        })
    }

    #[inline]
    #[track_caller]
    #[must_use]
    pub fn get(self) -> T {
        self.read("get", true)
    }

    #[inline]
    #[track_caller]
    #[must_use]
    pub fn peek(self) -> T {
        self.read("peek", false)
    }

    #[inline]
    #[must_use]
    pub(crate) fn try_peek(self) -> Option<T> {
        with_rt(|rt| {
            let value = &rt.signals.get(self.key)?.value;
            value.downcast_ref::<T>().cloned()
        })
    }
}

fn track_key(rt: &mut RuntimeState, key: SignalKey) {
    let Some(effect) = rt.current_effect else {
        return;
    };
    let Some(eff) = rt.effects.get_mut(effect) else {
        return;
    };
    if eff.prev_deps.get(eff.prev_dep_cursor) == Some(&key) {
        eff.prev_dep_cursor += 1;
        eff.deps.push(key);
        return;
    }
    if eff.deps.last() == Some(&key) || eff.deps.contains(&key) {
        return;
    }
    eff.deps.push(key);
    if !eff.prev_deps.contains(&key)
        && let Some(slot) = rt.signals.get_mut(key)
    {
        slot.subscribers.push(effect);
    }
}

impl<T: 'static> Signal<T> {
    #[track_caller]
    pub(crate) fn track(self) {
        let caller = Location::caller();
        if !self.try_track() {
            disposed_signal("track", caller);
        }
    }

    pub(crate) fn try_track(self) -> bool {
        with_rt(|rt| {
            if rt.signals.get(self.key).is_none() {
                return false;
            }
            track_key(rt, self.key);
            true
        })
    }

    #[track_caller]
    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let caller = Location::caller();
        let taken = with_rt(|rt| {
            let taken = take_value(rt, self.key, "with", caller);
            track_key(rt, self.key);
            taken
        });
        f(UpdateGuard::new(self.key, taken).value_ref::<T>())
    }
}

#[cfg(test)]
#[path = "sendable_tests.rs"]
mod sendable_tests;
