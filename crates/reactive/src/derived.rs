//! Derived (computed) reactive values.
//!
//! `derived(f)` registers a closure whose result is stored in a
//! backing signal. The closure runs once on registration and again
//! whenever any signal it read changes. Reads of the returned signal
//! subscribe the current effect, just like a regular signal.
//!
//! Two variants exist:
//!
//! - [`derived`] — always notifies dependents, even when the newly
//!   computed value is `==` the previous value. Trait bound is
//!   `T: Clone + 'static`.
//! - [`derived_eq`] — memoizes by `PartialEq`: a re-run that produces
//!   an equal value does *not* notify dependents. Trait bound is
//!   `T: Clone + PartialEq + 'static`.
//!
//! This mirrors Leptos 0.5+'s `create_memo` (which requires
//! `PartialEq`) and Solid.js's `createMemo` (which memoizes by
//! identity). We split the two so callers whose `T` is not
//! `PartialEq` (closures, non-comparable newtypes) can still build
//! reactive computations.
//!
//! # Ownership
//!
//! The backing signal is parented to the scope that called
//! [`derived`]/[`derived_eq`], not to the internal tracking effect's
//! per-run owner. That means the signal survives every re-run of the
//! tracking effect and is disposed only when the enclosing scope is
//! disposed. Cleanups registered from inside `f` live on the
//! tracking effect's per-run owner and fire before each re-run —
//! the standard effect-owner contract.
//!
//! # First-run contract
//!
//! `f` is called exactly once per dependency update, including the
//! first run. The backing signal is populated during that first run
//! (which is synchronous inside `effect(...)`), so by the time
//! [`derived`]/[`derived_eq`] returns, `.get()` on the returned
//! signal yields a valid value.

use core::panic::Location;
use std::cell::Cell;
use std::rc::Rc;

use smallvec::SmallVec;

use crate::effect::effect;
use crate::runtime::{OwnerKey, SignalSlot, with_rt};
use crate::signal::Signal;

/// Create a signal parented to `owner` rather than `current_owner`.
///
/// Used to attach a derived's backing signal to the enclosing scope
/// instead of the tracking effect's per-run owner. Panics if `owner`
/// is not alive in the arena.
fn signal_in_owner<T: 'static>(
    initial: T,
    owner: OwnerKey,
    location: &'static Location<'static>,
) -> Signal<T> {
    let key = with_rt(|rt| {
        assert!(
            rt.owners.contains(owner),
            "derived signal parent owner was disposed (at {location})",
        );
        let key = rt.signals.insert(SignalSlot {
            owner,
            value: Box::new(initial),
            subscribers: SmallVec::new(),
            location,
        });
        rt.owners
            .get_mut(owner)
            .expect("owner validated above")
            .signals
            .push(key);
        key
    });
    Signal::from_key(key)
}

/// Grab the current reactive owner, panicking if none is active.
fn current_owner_or_panic(caller: &'static Location<'static>) -> OwnerKey {
    with_rt(|rt| {
        rt.current_owner
            .unwrap_or_else(|| crate::diagnostic::no_scope("derived()", caller))
    })
}

/// Register a derived (computed) reactive value.
///
/// `f` is called once immediately to produce the initial value;
/// during that call, signals read inside `f` subscribe the derived's
/// internal tracking effect. Subsequent changes to any of those
/// signals re-run `f` and write the new value into the backing
/// signal, which then notifies its own subscribers.
///
/// This variant does **not** memoize: every re-run of `f` writes the
/// new value even when it is equal to the previous value. Use
/// [`derived_eq`] if `T: PartialEq` and you want re-runs that produce
/// an equal value to be silent.
///
/// `f` is guaranteed to be called exactly once per dependency update,
/// including the first registration — callers that rely on this
/// (allocation counters, instrumentation) can depend on it.
///
/// # Panics
///
/// Panics if called outside a reactive scope. Reading the returned
/// signal after its owning scope has been disposed panics in the
/// same way as reading any disposed signal.
#[must_use]
#[track_caller]
pub fn derived<T, F>(mut f: F) -> Signal<T>
where
    T: Clone + 'static,
    F: FnMut() -> T + 'static,
{
    let caller = Location::caller();
    let parent_owner = current_owner_or_panic(caller);

    // `Rc<Cell<Option<Signal<T>>>>` is cheap: `Signal<T>` is `Copy`
    // and the whole thing stays on one thread.
    let holder: Rc<Cell<Option<Signal<T>>>> = Rc::new(Cell::new(None));
    let holder_inside = Rc::clone(&holder);

    // The tracking effect. On its first run we materialise the
    // backing signal (parented to `parent_owner`, not the effect's
    // per-run owner). On subsequent runs we write the new value
    // unconditionally — no memoization in this variant.
    effect(move || {
        let value = f();
        match holder_inside.get() {
            None => {
                let sig = signal_in_owner(value, parent_owner, caller);
                holder_inside.set(Some(sig));
            }
            Some(sig) => sig.set_always(value),
        }
    });

    holder.get().expect(
        "derived's tracking effect populates the backing signal on its synchronous first run",
    )
}

/// Register a memoizing derived (computed) reactive value.
///
/// Same semantics as [`derived`], but re-runs of `f` that produce a
/// value equal (by `PartialEq`) to the previous one do **not** notify
/// dependents. This is the direct analogue of Leptos 0.5+'s
/// `create_memo` and Solid's `createMemo`.
///
/// `f` is called exactly once per dependency update, including the
/// first registration.
///
/// # Panics
///
/// Panics if called outside a reactive scope.
#[must_use]
#[track_caller]
pub fn derived_eq<T, F>(mut f: F) -> Signal<T>
where
    T: Clone + PartialEq + 'static,
    F: FnMut() -> T + 'static,
{
    let caller = Location::caller();
    let parent_owner = current_owner_or_panic(caller);

    let holder: Rc<Cell<Option<Signal<T>>>> = Rc::new(Cell::new(None));
    let holder_inside = Rc::clone(&holder);

    effect(move || {
        let value = f();
        match holder_inside.get() {
            None => {
                let sig = signal_in_owner(value, parent_owner, caller);
                holder_inside.set(Some(sig));
            }
            Some(sig) => {
                // Compare the newly computed value against the stored
                // one without cloning the stored side out and without
                // subscribing the tracking effect to its own backing
                // signal. Safe to reach into the arena directly here:
                // the memoization check never runs user code while the
                // borrow is held.
                let key = sig.key_for_derived();
                let is_equal = with_rt(|rt| {
                    rt.signals.get(key).is_some_and(|slot| {
                        slot.value
                            .downcast_ref::<T>()
                            .expect("derived signal type invariant")
                            == &value
                    })
                });
                if !is_equal {
                    sig.set(value);
                }
            }
        }
    });

    holder.get().expect(
        "derived's tracking effect populates the backing signal on its synchronous first run",
    )
}
