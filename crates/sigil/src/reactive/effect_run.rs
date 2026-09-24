use core::panic::Location;

use crate::reactive::owner_dispose::{dispose_owner, remove_effect, take_deferred_dispose};
use crate::reactive::runtime::{EffectKey, OwnerKey, OwnerSlot, RuntimeState, with_rt};

struct RunGuard {
    key: EffectKey,
    prev_owner: Option<OwnerKey>,
    prev_effect: Option<EffectKey>,
    f: Option<Box<dyn FnMut()>>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        let mut f = self.f.take();
        let run_owner = with_rt(|rt| {
            let deferred = take_deferred_dispose(rt, self.key);
            let mut run_owner = None;
            if std::thread::panicking() {
                if rt.panic_origin.is_none()
                    && let Some(slot) = rt.effects.get(self.key)
                {
                    rt.panic_origin = Some((self.key, slot.name, slot.location));
                }
                if rt.panic_origin.is_some_and(|(key, ..)| key == self.key) {
                    rt.running.remove(&self.key);
                    run_owner = remove_effect(rt, self.key);
                } else if !deferred {
                    f = restore_interrupted(rt, self.key, f.take());
                }
            } else {
                reconcile_deps(rt, self.key);
                if !deferred && let Some(slot) = rt.effects.get_mut(self.key) {
                    slot.f = f.take();
                }
            }
            rt.current_owner = self.prev_owner;
            rt.current_effect = self.prev_effect;
            rt.running.remove(&self.key);
            if deferred && run_owner.is_none() {
                run_owner = remove_effect(rt, self.key);
            }
            run_owner
        });
        drop(f);
        if let Some(run_owner) = run_owner {
            dispose_owner(run_owner);
        }
    }
}

fn restore_interrupted(
    rt: &mut RuntimeState,
    key: EffectKey,
    f: Option<Box<dyn FnMut()>>,
) -> Option<Box<dyn FnMut()>> {
    let Some((partial, old)) = rt.effects.get_mut(key).map(|slot| {
        (
            core::mem::take(&mut slot.deps),
            core::mem::take(&mut slot.prev_deps),
        )
    }) else {
        return f;
    };
    for dep in partial {
        if old.contains(&dep) {
            continue;
        }
        if let Some(sig) = rt.signals.get_mut(dep)
            && let Some(pos) = sig.subscribers.iter().position(|k| *k == key)
        {
            sig.subscribers.swap_remove(pos);
        }
    }
    let Some(slot) = rt.effects.get_mut(key) else {
        return f;
    };
    slot.deps = old;
    slot.prev_dep_cursor = 0;
    slot.f = f;
    None
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

enum RunSetup {
    Skip,
    AbortClearRunning,
    Go {
        f: Box<dyn FnMut()>,
        prev_owner: Option<OwnerKey>,
        prev_effect: Option<EffectKey>,
        prev_run_owner: Option<OwnerKey>,
    },
}

fn prune_dead_deps(rt: &mut RuntimeState, key: EffectKey) {
    let Some(current) = rt.effects.get(key) else {
        return;
    };
    if current.deps.iter().all(|dep| rt.signals.contains(*dep)) {
        return;
    }
    let Some(mut deps) = rt
        .effects
        .get_mut(key)
        .map(|s| core::mem::take(&mut s.deps))
    else {
        return;
    };
    deps.retain(|dep| rt.signals.contains(*dep));
    if let Some(slot) = rt.effects.get_mut(key) {
        slot.deps = deps;
    }
}

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
        rt.owners.remove(run_owner);
        None
    }
}

#[inline]
fn run_setup(key: EffectKey) -> RunSetup {
    with_rt(|rt| {
        if !rt.effects.contains(key) {
            return RunSetup::Skip;
        }
        if rt.running.contains(&key) {
            let loc = rt.effects.get(key).map(|e| e.location);
            crate::reactive::diagnostic::reactive_cycle(loc);
            return RunSetup::Skip;
        }
        if rt.effects.get(key).is_some_and(|slot| slot.f.is_none()) {
            return RunSetup::Skip;
        }
        prune_dead_deps(rt, key);
        rt.running.insert(key);
        go_setup(rt, key)
    })
}

fn go_setup(rt: &mut RuntimeState, key: EffectKey) -> RunSetup {
    if let Some(effect) = rt.effects.get_mut(key) {
        debug_assert!(effect.prev_deps.is_empty());
        effect.prev_deps = core::mem::take(&mut effect.deps);
        effect.prev_dep_cursor = 0;
    }

    let prev_run_owner = rt.effects.get_mut(key).and_then(|e| e.run_owner.take());

    let Some((parent_owner, effect_location)) = rt.effects.get(key).map(|e| (e.owner, e.location))
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

    let mut guard = RunGuard {
        key,
        prev_owner,
        prev_effect,
        f: Some(f),
    };

    if let Some(prev) = prev_run_owner {
        dispose_owner(prev);
        #[cfg(debug_assertions)]
        {
            let (name, location) = with_rt(|rt| {
                rt.effects
                    .get(key)
                    .map_or(("<effect>", Location::caller()), |e| (e.name, e.location))
            });
            crate::reactive::debug::run_effect_leak_check(prev, name, location);
        }
    }

    guard.f.as_mut().expect("closure just stored in guard")();
    drop(guard);
    crate::reactive::effect::run_deferred(key);

    trace_effect_run(key);
}

fn trace_effect_run(key: EffectKey) {
    if !crate::reactive::trace::trace_active() {
        return;
    }
    let trace_inputs = with_rt(|rt| {
        rt.effects
            .get(key)
            .map(|slot| (slot.name, slot.location, slot.deps.len(), rt.pending.len()))
    });
    if let Some((name, location, reads, writes_pending)) = trace_inputs {
        crate::reactive::trace::trace_effect_fire(name, location, reads, writes_pending);
    }
}
