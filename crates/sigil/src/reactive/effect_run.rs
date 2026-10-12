use crate::reactive::diagnostic::{Cycle, reactive_cycle};
use crate::reactive::owner_dispose::{dispose_owner, unsubscribe};
use crate::reactive::runtime::{EffectKey, OwnerKey, RuntimeState, with_rt};

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
            let deferred = rt.take_deferred_dispose(self.key);
            let mut run_owner = None;
            if std::thread::panicking() {
                if rt.panic_origin.is_none()
                    && let Some(slot) = rt.effects.get(self.key)
                {
                    rt.panic_origin = Some((self.key, slot.name, slot.location));
                }
                if rt.panic_origin.is_some_and(|(key, ..)| key == self.key) {
                    rt.running.remove(&self.key);
                    run_owner = rt.remove_effect(self.key);
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
                run_owner = rt.remove_effect(self.key);
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
        unsubscribe(&mut rt.signals, dep, key);
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
        unsubscribe(&mut rt.signals, dep, key);
    }
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

fn run_setup(rt: &mut RuntimeState, key: EffectKey) -> Option<(RunGuard, Option<OwnerKey>)> {
    let slot = rt.effects.get(key)?;
    if rt.running.contains(&key) {
        reactive_cycle(Cycle::ReEntry(Some(slot.location)));
        return None;
    }
    slot.f.as_ref()?;
    prune_dead_deps(rt, key);
    rt.running.insert(key);
    let setup = go_setup(rt, key);
    if setup.is_none() {
        rt.running.remove(&key);
    }
    setup
}

fn go_setup(rt: &mut RuntimeState, key: EffectKey) -> Option<(RunGuard, Option<OwnerKey>)> {
    let effect = rt.effects.get_mut(key)?;
    debug_assert!(effect.prev_deps.is_empty());
    effect.prev_deps = core::mem::take(&mut effect.deps);
    effect.prev_dep_cursor = 0;
    let prev_run_owner = effect.run_owner.take();
    let (parent, location) = (effect.owner, effect.location);
    if !rt.owners.contains(parent) {
        return None;
    }
    let run_owner = rt.insert_owner("<effect-run>", Some(parent), location);
    let slot = rt
        .effects
        .get_mut(key)
        .expect("invariant: effect slot present during setup");
    slot.run_owner = Some(run_owner);
    let f = slot
        .f
        .take()
        .expect("invariant: effect closure present outside a run");
    let guard = RunGuard {
        key,
        prev_owner: rt.current_owner.replace(run_owner),
        prev_effect: rt.current_effect.replace(key),
        f: Some(f),
    };
    Some((guard, prev_run_owner))
}

pub(crate) fn run_effect(key: EffectKey) {
    let Some((mut guard, prev_run_owner)) = with_rt(|rt| run_setup(rt, key)) else {
        return;
    };
    if let Some(prev) = prev_run_owner {
        dispose_owner(prev);
        #[cfg(debug_assertions)]
        {
            let (name, location) = with_rt(|rt| {
                rt.effects
                    .get(key)
                    .map_or(("<effect>", core::panic::Location::caller()), |e| {
                        (e.name, e.location)
                    })
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
