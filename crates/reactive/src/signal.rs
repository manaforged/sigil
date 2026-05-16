//! Reactive signals — struct definitions, constructor, and read path.
//!
//! `Signal<T>` is a `Copy` handle into the runtime's signal arena.
//! Reads subscribe the currently-running effect to the signal;
//! writes schedule all subscribed effects. Both operations are
//! main-thread only — `Signal` is `!Send` and `!Sync`.
//!
//! Write methods (`set`, `update`) and their helpers live in
//! [`crate::signal_write`] to keep each file under the 400-line cap.

use core::marker::PhantomData;
use core::panic::Location;

use smallvec::SmallVec;

use crate::runtime::{EffectKey, SignalKey, SignalSlot, runtime_id, with_rt};

/// A reactive signal carrying a value of type `T`.
///
/// `Signal<T>` is `Copy`. It is an arena handle, not the value
/// itself. Holding a `Signal<T>` after its owner has been disposed
/// is safe but reads and writes panic with a stale-handle message.
pub struct Signal<T: 'static> {
    pub(crate) key: SignalKey,
    // !Send + !Sync because the arena is thread-local and values
    // may not themselves be `Send`.
    _not_send: PhantomData<*const T>,
}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Signal<T> {}

impl<T: 'static> Signal<T> {
    /// Wrap a raw arena key as a `Signal<T>`.
    ///
    /// Used by the `derived` module to build signals via direct arena
    /// insertion when the caller needs to parent the signal to an
    /// owner other than `current_owner` (i.e. the enclosing scope,
    /// not the tracking effect's per-run owner).
    pub(crate) const fn from_key(key: SignalKey) -> Self {
        Self {
            key,
            _not_send: PhantomData,
        }
    }

    /// The raw arena key backing this signal.
    ///
    /// Exposed to the `derived` module for its memoization equality
    /// check, which needs to read the current stored value via
    /// `with_rt` rather than through the public `peek` (to avoid
    /// borrow nesting). No external API exposes this.
    pub(crate) const fn key_for_derived(self) -> SignalKey {
        self.key
    }

    /// Erase the `!Send` marker so the signal's arena identity can be
    /// carried into a background task and re-hydrated back into a
    /// `Signal<T>` on the main thread.
    ///
    /// Used by `sigil-async::spawn_to_signal` to publish a
    /// background-produced value into a pre-existing main-thread
    /// signal: the main thread creates the signal and extracts its
    /// sendable identity; the background future owns the sendable
    /// handle and posts it back through `dispatch_to_owner`, where the
    /// main-thread closure re-hydrates and writes it.
    ///
    /// Re-hydration is only safe on the thread that owns the reactive
    /// runtime — i.e., inside a `dispatch_to_owner` closure or on the
    /// main thread in general. The re-hydrated `Signal<T>` is still
    /// subject to the normal stale-handle panic if its owner has been
    /// disposed.
    ///
    /// Marked `#[doc(hidden)]` because the only legitimate consumer
    /// is `sigil-async`. User code that thinks it needs this is almost
    /// certainly doing something wrong — use `spawn_to_signal` or
    /// `dispatch_to_owner` directly.
    #[doc(hidden)]
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

/// `Send + Sync` identity of a `Signal<T>`, re-hydratable on the main
/// thread.
///
/// See [`Signal::into_sendable`].
///
/// The wrapper carries three fields: the arena `index` and
/// `generation` of the underlying signal slot, plus the `runtime_id`
/// of the reactive runtime the signal was minted in. On re-hydration
/// [`SendableSignal::into_signal`] checks the stored `runtime_id`
/// against the current thread's runtime and panics on mismatch —
/// without that check a handle posted to the wrong thread would
/// either look disposed (best case) or silently alias into an
/// unrelated signal that happens to share the `(index, generation)`
/// pair in that thread's arena (worst case).
#[doc(hidden)]
pub struct SendableSignal<T: 'static> {
    index: u32,
    generation: u32,
    /// Identity of the runtime that produced this handle. Compared
    /// against the current thread's runtime id on re-hydration so
    /// cross-thread leaks panic with an actionable message rather
    /// than silently aliasing into an unrelated signal slot.
    runtime_id: u64,
    // `fn() -> T` is covariant in `T` and is `Send + Sync` regardless
    // of whether `T` is. This mirrors `Signal<T>`'s phantom, modulo
    // the raw pointer that forces `!Send` on the main-thread handle.
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
    /// Re-hydrate into a main-thread `Signal<T>` handle.
    ///
    /// The returned handle is equivalent to the one the sendable
    /// form was created from. Must be called on the thread that
    /// owns the reactive runtime the signal was created in.
    ///
    /// # Panics
    ///
    /// Panics if called on a thread whose reactive runtime differs
    /// from the one the sendable was minted in.
    #[doc(hidden)]
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

/// Create a new signal owned by the current reactive scope.
///
/// # Panics
///
/// Panics if called outside any reactive scope. Use
/// [`Runtime::scope`](crate::Runtime::scope) or
/// [`Owner::with`](crate::Owner::with) to establish one.
#[must_use]
#[track_caller]
pub fn signal<T: 'static>(initial: T) -> Signal<T> {
    let location = Location::caller();
    let key = with_rt(|rt| {
        let owner = rt
            .current_owner
            .unwrap_or_else(|| crate::diagnostic::no_scope("signal()", location));
        // Validate `current_owner` is still alive BEFORE inserting
        // into the arena, so a stale current_owner does not leak
        // a detached signal slot.
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
    /// Read the signal and subscribe the current effect (if any).
    ///
    /// # Panics
    ///
    /// Panics if the signal has been disposed or if the stored value
    /// is not of type `T`.
    ///
    /// `T::clone` runs while the runtime is borrowed. A
    /// pathological `T::clone` that calls back into the runtime will
    /// panic with `BorrowMutError`. Keep `Clone` on signal values cheap
    /// and non-reactive.
    #[inline]
    #[track_caller]
    #[must_use]
    pub fn get(self) -> T {
        let caller = Location::caller();
        with_rt(|rt| {
            let Some(slot) = rt.signals.get(self.key) else {
                crate::diagnostic::disposed_signal("get", caller);
            };
            assert!(
                !slot.value.is::<crate::signal_write::UpdateInProgress>(),
                "Signal::get on a signal whose own update closure is still running (at {caller})"
            );
            let value = slot
                .value
                .downcast_ref::<T>()
                .expect("signal type mismatch")
                .clone();
            if let Some(effect) = rt.current_effect {
                let should_subscribe = if let Some(eff) = rt.effects.get_mut(effect) {
                    if eff
                        .prev_deps
                        .get(eff.prev_dep_cursor)
                        .is_some_and(|&dep| dep == self.key)
                    {
                        eff.prev_dep_cursor += 1;
                        eff.deps.push(self.key);
                        false
                    } else if eff.deps.last().is_some_and(|&dep| dep == self.key)
                        || eff.deps.contains(&self.key)
                    {
                        false
                    } else {
                        let already_subscribed = eff.prev_deps.contains(&self.key);
                        eff.deps.push(self.key);
                        !already_subscribed
                    }
                } else {
                    false
                };
                if should_subscribe && let Some(sub_slot) = rt.signals.get_mut(self.key) {
                    sub_slot.subscribers.push(effect);
                }
            }
            value
        })
    }

    /// Read the signal *without* subscribing the current effect.
    ///
    /// # Panics
    ///
    /// Panics if the signal has been disposed.
    ///
    /// `T::clone` runs while the runtime is borrowed — same caveat
    /// as [`get`](Self::get).
    #[inline]
    #[track_caller]
    #[must_use]
    pub fn peek(self) -> T {
        let caller = Location::caller();
        with_rt(|rt| {
            let Some(slot) = rt.signals.get(self.key) else {
                crate::diagnostic::disposed_signal("peek", caller);
            };
            assert!(
                !slot.value.is::<crate::signal_write::UpdateInProgress>(),
                "Signal::peek on a signal whose own update closure is still running (at {caller})"
            );
            slot.value
                .downcast_ref::<T>()
                .expect("signal type mismatch")
                .clone()
        })
    }
}

// `EffectKey` is used in `get` but only through `rt.current_effect` /
// `rt.effects` — rustc would warn about an unused import otherwise.
// The import is load-bearing for the type checker on the `if let
// Some(effect): Option<EffectKey>` binding.
const _: () = {
    let _ = core::mem::size_of::<EffectKey>();
};
