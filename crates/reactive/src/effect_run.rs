//! Effect execution machinery — run loop, RAII guard, and setup.
//!
//! Separated from the public `effect` registration API to keep each
//! file under the 400-line cap. Nothing here is re-exported; items
//! are `pub(crate)` or private.

use core::panic::Location;

use crate::owner_dispose::dispose_owner;
use crate::runtime::{EffectKey, OwnerKey, OwnerSlot, RuntimeState, with_rt};

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
struct RunGuard {
    key: EffectKey,
    prev_owner: Option<OwnerKey>,
    prev_effect: Option<EffectKey>,
    f: Option<Box<dyn FnMut()>>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        let f = if std::thread::panicking() {
            None
        } else {
            self.f.take()
        };
        with_rt(|rt| {
            reconcile_deps(rt, self.key);
            rt.current_owner = self.prev_owner;
            rt.current_effect = self.prev_effect;
            rt.running.remove(&self.key);
            if let Some(f) = f
                && let Some(slot) = rt.effects.get_mut(self.key)
            {
                slot.f = Some(f);
            }
            // If the slot is gone the closure drops here.
        });
    }
}

fn reconcile_deps(rt: &mut RuntimeState, key: EffectKey) {
    let Some((old_deps, new_deps)) = rt
        .effects
        .get_mut(key)
        .map(|slot| (core::mem::take(&mut slot.prev_deps), slot.deps.clone()))
    else {
        return;
    };

    if old_deps == new_deps {
        return;
    }

    for dep in old_deps {
        if new_deps.contains(&dep) {
            continue;
        }
        if let Some(signal) = rt.signals.get_mut(dep)
            && let Some(pos) = signal.subscribers.iter().position(|k| *k == key)
        {
            signal.subscribers.swap_remove(pos);
        }
    }
}

/// Outcome of the fused `run_effect` setup pass.
///
/// Returned from the first `with_rt` borrow so the caller can drive
/// the out-of-borrow work (dispose previous run-owner, run closure)
/// without re-entering the runtime for each step.
enum RunSetup {
    /// Effect slot is gone or running was already set; skip this run.
    Skip,
    /// Parent owner or effect slot vanished between checkpoints;
    /// running was set and must be cleared by the caller.
    AbortClearRunning,
    /// Setup succeeded.
    Go {
        f: Box<dyn FnMut()>,
        prev_owner: Option<OwnerKey>,
        prev_effect: Option<EffectKey>,
        prev_run_owner: Option<OwnerKey>,
    },
}

/// Create a fresh run-owner for an effect execution, wire it to the
/// parent, and record it on the effect slot.
///
/// Returns `None` if the effect slot was removed between the parent
/// lookup and the owner insertion (defensive; should not happen
/// while we hold the runtime borrow).
fn make_effect_run_owner(
    rt: &mut RuntimeState,
    key: EffectKey,
    parent_owner: OwnerKey,
    effect_location: &'static Location<'static>,
) -> Option<OwnerKey> {
    let index_in_parent = rt
        .owners
        .get(parent_owner)
        .map(|parent| parent.children.len());
    let run_owner = rt.owners.insert(OwnerSlot {
        parent: Some(parent_owner),
        index_in_parent,
        children: Vec::new(),
        signals: Vec::new(),
        effects: Vec::new(),
        entities: Vec::new(),
        observers: Vec::new(),
        cleanups: Vec::new(),
        name: "<effect-run>",
        location: effect_location,
    });
    if let Some(p) = rt.owners.get_mut(parent_owner) {
        p.children.push(run_owner);
    }
    if let Some(slot) = rt.effects.get_mut(key) {
        slot.run_owner = Some(run_owner);
        Some(run_owner)
    } else {
        // Effect vanished while we held the borrow — clean up the
        // orphan owner we just created.
        rt.owners.remove(run_owner);
        None
    }
}

/// Fused setup pass for `run_effect` — batches all pre-run work into
/// a single `with_rt` borrow so that the out-of-borrow work
/// (previous run-owner disposal, user closure execution) does not
/// need to re-enter the runtime.
#[inline]
fn run_setup(key: EffectKey) -> RunSetup {
    with_rt(|rt| {
        if !rt.effects.contains(key) {
            return RunSetup::Skip;
        }
        if rt.running.contains(&key) {
            let loc = rt.effects.get(key).map(|e| e.location);
            crate::diagnostic::reactive_cycle(loc);
            return RunSetup::Skip;
        }
        if rt.effects.get(key).is_some_and(|slot| slot.f.is_none()) {
            return RunSetup::Skip;
        }
        rt.running.insert(key);

        // Rotate deps: move current → prev so RunGuard can reconcile.
        if let Some(effect) = rt.effects.get_mut(key) {
            debug_assert!(effect.prev_deps.is_empty());
            effect.prev_deps = core::mem::take(&mut effect.deps);
            effect.prev_dep_cursor = 0;
        }

        let prev_run_owner = rt.effects.get_mut(key).and_then(|e| e.run_owner.take());

        let Some((parent_owner, effect_location)) =
            rt.effects.get(key).map(|e| (e.owner, e.location))
        else {
            return RunSetup::AbortClearRunning;
        };
        if !rt.owners.contains(parent_owner) {
            return RunSetup::AbortClearRunning;
        }

        let Some(run_owner) = make_effect_run_owner(rt, key, parent_owner, effect_location) else {
            return RunSetup::AbortClearRunning;
        };

        let slot = rt
            .effects
            .get_mut(key)
            .expect("invariant: slot present after make_effect_run_owner");
        let f = slot
            .f
            .take()
            .expect("invariant: effect closure present outside a run");
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

pub(crate) fn run_effect(key: EffectKey) {
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
    // recurses through `with_rt`, so it cannot be fused into setup.
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

    let mut guard = RunGuard {
        key,
        prev_owner,
        prev_effect,
        f: Some(f),
    };
    guard.f.as_mut().expect("closure just stored in guard")();
    drop(guard);

    if crate::trace::trace_active() {
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
