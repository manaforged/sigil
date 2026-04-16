//! Effects — side effects that re-run on signal changes.
//!
//! An effect runs once immediately, subscribing to every signal
//! it reads via `Signal::get`. When any subscribed signal is
//! written, the effect is scheduled to re-run. Re-runs clear old
//! subscriptions so dependency sets shrink naturally when control
//! flow changes.
//!
//! Writes inside `batch(|| ...)` coalesce: the effect runs once at
//! the end of the batch no matter how many subscribed signals
//! changed.
//!
//! Reactive cycles panic in debug builds and emit an actionable
//! warning plus skip the re-entrant run in release builds.
//!
//! # Effect-owner model
//!
//! Per Leptos 0.5+ / Solid: every effect is itself a reactive owner.
//! On each run we:
//!
//! 1. Dispose the previous run's effect-owner (firing its cleanups
//!    and tearing down nested signals/effects).
//! 2. Create a fresh child owner under the scope where `effect()`
//!    was called.
//! 3. Run the closure with `current_owner = run_owner` and
//!    `current_effect = effect_key`.
//!
//! This means `on_cleanup` registered inside an effect fires before
//! the next re-run, and signals created inside an effect are
//! disposed on re-run.
//!
//! Run-loop internals live in [`crate::effect_run`].

use core::panic::Location;

use smallvec::SmallVec;

use crate::effect_run::run_effect;
use crate::runtime::{EffectKey, EffectSlot, with_rt};

/// Register an effect owned by the current reactive scope.
///
/// The effect runs once immediately on registration. Every signal
/// read during that run becomes a dependency; subsequent writes to
/// any of those signals re-run the effect.
///
/// The effect is registered with the debug name `"<unnamed>"`. Use
/// [`effect_named`] if the call site has a meaningful label
/// (e.g., `"Button.hover_style"`) — the name surfaces in the
/// `SIGIL_TRACE_EFFECTS` trace log, the inspector, and future
/// debug tooling. See DR-005 addendum items 1 and 6.
///
/// # Panics
///
/// Panics if called outside a reactive scope, or if this effect
/// re-enters itself through a signal write in debug builds.
#[track_caller]
pub fn effect<F: FnMut() + 'static>(f: F) {
    effect_named_impl("<unnamed>", f, Location::caller());
}

/// Register a named effect owned by the current reactive scope.
///
/// Same runtime behaviour as [`effect`]; the supplied `name` is
/// recorded on the effect slot and emitted by the trace log, the
/// owner-tree dump, and future debug tooling. The name is
/// `&'static str` — pass string literals (`"Button.hover_style"`) or
/// statically-known identifiers.
///
/// # Panics
///
/// Panics under the same conditions as [`effect`].
#[track_caller]
pub fn effect_named<F: FnMut() + 'static>(name: &'static str, f: F) {
    effect_named_impl(name, f, Location::caller());
}

/// Shared registration path used by both [`effect`] and
/// [`effect_named`]. Takes `location` explicitly so the caller can
/// capture `Location::caller()` at the outer `#[track_caller]` frame.
fn effect_named_impl<F: FnMut() + 'static>(
    name: &'static str,
    f: F,
    location: &'static Location<'static>,
) {
    let key = with_rt(|rt| {
        let owner = rt
            .current_owner
            .unwrap_or_else(|| crate::diagnostic::no_scope("effect()", location));
        // Validate `current_owner` is still alive BEFORE inserting
        // into the effects arena, so a stale current_owner does not
        // leak a detached effect slot.
        assert!(
            rt.owners.contains(owner),
            "effect() with a disposed current owner (at {location})"
        );
        let key = rt.effects.insert(EffectSlot {
            owner,
            run_owner: None,
            f: Box::new(f),
            deps: SmallVec::new(),
            prev_deps: SmallVec::new(),
            prev_dep_cursor: 0,
            location,
            name,
        });
        rt.owners
            .get_mut(owner)
            .expect("current owner slot missing")
            .effects
            .push(key);
        key
    });
    run_effect(key);
}

/// Schedule a batch of effects to run.
///
/// Outside a batch, each effect runs synchronously. Inside a
/// `batch(|| ...)` call, effects are queued and drained once on
/// batch exit, deduplicated by effect id.
///
/// Fused into a single `with_rt` on the fast path: we check
/// `batch_depth` and either enqueue (all inside the borrow) or
/// return the effects so the caller runs them synchronously after
/// the borrow releases (since `run_effect` re-enters the runtime).
pub(crate) fn schedule_effects(effects: SmallVec<[EffectKey; 4]>) {
    let to_run = with_rt(|rt| {
        if rt.batch_depth > 0 {
            for e in effects {
                if rt.pending_set.insert(e) {
                    rt.pending.push_back(e);
                }
            }
            None
        } else {
            Some(effects)
        }
    });
    if let Some(ready) = to_run {
        for e in ready {
            run_effect(e);
        }
    }
}

/// Drain the pending-effect queue.
///
/// Called by `batch` at batch exit.
pub(crate) fn drain_pending() {
    loop {
        let next = with_rt(|rt| {
            let next = rt.pending.pop_front();
            if let Some(key) = next {
                rt.pending_set.remove(&key);
            }
            next
        });
        let Some(key) = next else {
            return;
        };
        run_effect(key);
    }
}
