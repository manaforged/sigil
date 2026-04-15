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

use core::panic::Location;

use smallvec::SmallVec;

use crate::owner::dispose_owner;
use crate::runtime::{EffectKey, EffectSlot, OwnerKey, OwnerSlot, with_rt};

/// Register an effect owned by the current reactive scope.
///
/// The effect runs once immediately on registration. Every signal
/// read during that run becomes a dependency; subsequent writes to
/// any of those signals re-run the effect.
///
/// The effect is registered with the debug name `"<unnamed>"`. Use
/// [`effect_named`] if the call site has a meaningful label
/// (e.g., `"Button.hover_style"`) — the name surfaces in the
/// `RUST_UI_TRACE_EFFECTS` trace log, the inspector, and future
/// debug tooling. See DR-005 addendum items 1 and 6.
///
/// # Panics
///
/// Panics if called outside a reactive scope, or if this effect
/// re-enters itself through a signal write (reactive cycle).
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
                if !rt.pending.contains(&e) {
                    rt.pending.push_back(e);
                }
            }
            None
        } else {
            Some(effects)
        }
    });
    if let Some(effects) = to_run {
        for e in effects {
            run_effect(e);
        }
    }
}

/// Drain the pending-effect queue.
///
/// Called by `batch` at batch exit.
pub(crate) fn drain_pending() {
    loop {
        let next = with_rt(|rt| rt.pending.pop_front());
        let Some(key) = next else {
            return;
        };
        run_effect(key);
    }
}

/// RAII guard that restores runtime state at the end of an effect
/// run regardless of whether the user closure returned normally or
/// unwound.
///
/// On drop we:
/// - Restore `current_owner` and `current_effect` to their prior
///   values.
/// - Remove `key` from the `running` set so a subsequent schedule
///   for the same effect is not mis-detected as a cycle.
/// - Put the closure back into the slot (if it still exists). If
///   the effect was disposed mid-run (e.g. panic-driven cleanup
///   cascade) the boxed `FnMut` drops with its captured state.
///
/// `f` is stored on the guard so that a panic inside the user
/// closure still restores the slot's closure on unwind. During
/// the synchronous `f()` call we `take()` it and put it back
/// on normal return; on unwind the guard's `Drop` reaches the
/// same state by taking whatever is left (the `None` case drops
/// nothing, which is what we want — the unwinding frame already
/// owns and drops the closure).
struct RunGuard {
    key: EffectKey,
    prev_owner: Option<OwnerKey>,
    prev_effect: Option<EffectKey>,
    f: Option<Box<dyn FnMut()>>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        let f = self.f.take();
        with_rt(|rt| {
            rt.current_owner = self.prev_owner;
            rt.current_effect = self.prev_effect;
            rt.running.remove(&self.key);
            if let Some(f) = f
                && let Some(slot) = rt.effects.get_mut(self.key)
            {
                slot.f = f;
            }
            // If the slot is gone the closure drops here.
        });
    }
}

/// Outcome of the fused `run_effect` setup pass.
///
/// Returned from the first `with_rt` borrow so the caller can drive
/// the out-of-borrow work (dispose previous run-owner, run closure)
/// without re-entering the runtime for each step.
enum RunSetup {
    /// Effect slot is gone or running was already set; skip this run.
    /// Caller must not touch the running set — either it was never
    /// set (slot-gone path), or the pre-existing `true` means another
    /// frame up the stack owns it and the cycle-check panic already
    /// fired.
    Skip,
    /// Parent owner or effect slot vanished between checkpoints;
    /// running was set and must be cleared by the caller.
    AbortClearRunning,
    /// Setup succeeded. `prev_run_owner` (if some) needs disposal
    /// before `f` runs; the caller must eventually hand `f`, the
    /// prev owner/effect values, and the key back to a `RunGuard`.
    Go {
        f: Box<dyn FnMut()>,
        prev_owner: Option<OwnerKey>,
        prev_effect: Option<EffectKey>,
        prev_run_owner: Option<OwnerKey>,
    },
}

/// Fused setup pass — a single `with_rt` that does everything the
/// effect needs before user code runs.
///
/// Inlined into `run_effect` so the one-borrow guarantee is preserved
/// at the call site. Kept as a separate function so the main driver
/// stays under the `clippy::too_many_lines` budget without an
/// `allow`.
#[inline]
fn run_setup(key: EffectKey) -> RunSetup {
    with_rt(|rt| {
        if !rt.effects.contains(key) {
            return RunSetup::Skip;
        }
        if rt.running.contains(&key) {
            let loc = rt.effects.get(key).map(|e| e.location);
            crate::diagnostic::reactive_cycle(loc);
        }
        rt.running.insert(key);

        // Clear old deps — take them off the effect slot, then
        // unsubscribe from each signal.
        let old_deps: SmallVec<[crate::runtime::SignalKey; 4]> = rt
            .effects
            .get_mut(key)
            .map(|e| core::mem::take(&mut e.deps))
            .unwrap_or_default();
        for dep in old_deps {
            if let Some(s) = rt.signals.get_mut(dep)
                && let Some(pos) = s.subscribers.iter().position(|k| *k == key)
            {
                s.subscribers.swap_remove(pos);
            }
        }

        // Take the previous run_owner for out-of-borrow disposal.
        let prev_run_owner = rt.effects.get_mut(key).and_then(|e| e.run_owner.take());

        // Look up parent owner + effect location for the new
        // run_owner slot.
        let Some((parent_owner, effect_location)) =
            rt.effects.get(key).map(|e| (e.owner, e.location))
        else {
            return RunSetup::AbortClearRunning;
        };
        if !rt.owners.contains(parent_owner) {
            return RunSetup::AbortClearRunning;
        }

        // Create run_owner, wire parent link, record on effect slot.
        let run_owner = rt.owners.insert(OwnerSlot {
            parent: Some(parent_owner),
            children: Vec::new(),
            signals: Vec::new(),
            effects: Vec::new(),
            cleanups: Vec::new(),
            name: "<effect-run>",
            location: effect_location,
        });
        if let Some(p) = rt.owners.get_mut(parent_owner) {
            p.children.push(run_owner);
        }
        let Some(slot) = rt.effects.get_mut(key) else {
            // Effect vanished (shouldn't happen: we hold the borrow,
            // but defend anyway) — clean up the orphan owner.
            rt.owners.remove(run_owner);
            return RunSetup::AbortClearRunning;
        };
        slot.run_owner = Some(run_owner);

        // Take the closure + save-then-switch current_owner/effect.
        let f = core::mem::replace(&mut slot.f, Box::new(|| {}));
        let prev_owner = rt.current_owner;
        let prev_effect = rt.current_effect;
        rt.current_owner = Some(run_owner);
        rt.current_effect = Some(key);

        RunSetup::Go {
            f,
            prev_owner,
            prev_effect,
            prev_run_owner,
        }
    })
}

fn run_effect(key: EffectKey) {
    // --- Fused setup pass (`with_rt` #1) -------------------------
    //
    // Everything the effect needs before we run user code, batched
    // into a single runtime borrow:
    //   - cycle check + mark running
    //   - clear old deps (iterate dep list, unsubscribe from signals)
    //   - take previous run_owner out of the slot (disposal below)
    //   - create a fresh run_owner and wire it to parent
    //   - save prev current_owner / current_effect, switch to the new
    //     run_owner and this effect
    //   - take the closure out of the slot
    //
    // Disposing the previous run_owner stays outside the borrow
    // because `dispose_owner` recursively re-enters `with_rt`.
    let (f, prev_owner, prev_effect, prev_run_owner) = match run_setup(key) {
        RunSetup::Skip => return,
        RunSetup::AbortClearRunning => {
            with_rt(|rt| {
                rt.running.remove(&key);
            });
            return;
        }
        RunSetup::Go {
            f,
            prev_owner,
            prev_effect,
            prev_run_owner,
        } => (f, prev_owner, prev_effect, prev_run_owner),
    };

    // Dispose the previous run-owner outside the borrow — this
    // recurses through `with_rt`, so it cannot be fused into the
    // setup pass above.
    if let Some(prev) = prev_run_owner {
        dispose_owner(prev);
        #[cfg(debug_assertions)]
        {
            let (name, location) = with_rt(|rt| {
                rt.effects
                    .get(key)
                    .map_or(("<effect>", Location::caller()), |e| (e.name, e.location))
            });
            crate::debug::run_effect_leak_check(prev, name, location);
        }
    }

    // --- Run user closure with RAII guard ------------------------
    //
    // The guard owns `f` for the duration so a panic inside `f`
    // still triggers the fused restore pass in `RunGuard::drop`
    // (`with_rt` #2). On the normal return path we take `f_local`
    // back out and let the now-populated guard drop.
    let mut guard = RunGuard {
        key,
        prev_owner,
        prev_effect,
        f: Some(f),
    };
    let mut f_local = guard.f.take().expect("closure just stored in guard");
    f_local();
    guard.f = Some(f_local);
    drop(guard);

    // Trace hook — see `crate::trace` for format and gating. Cheap
    // on the off path: `trace_active` is one relaxed atomic load
    // (plus one thread-local read in test builds). We only pay for
    // the runtime snapshot when at least one sink (stderr or the
    // test buffer) is listening.
    if crate::trace::trace_active() {
        // Effect may have been disposed mid-run (user closure
        // dropped its parent scope); skip in that case.
        let trace_inputs = with_rt(|rt| {
            rt.effects
                .get(key)
                .map(|slot| (slot.name, slot.location, slot.deps.len(), rt.pending.len()))
        });
        if let Some((name, location, reads, writes_pending)) = trace_inputs {
            crate::trace::trace_effect_fire(name, location, reads, writes_pending);
        }
    }
}
