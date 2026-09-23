use crate::reactive::runtime::{EffectKey, OwnerKey, RuntimeState, SignalKey, with_rt};

fn take_owner_resources(key: OwnerKey) -> (Vec<SignalKey>, Vec<EffectKey>) {
    with_rt(|rt| {
        rt.owners
            .get_mut(key)
            .map(|o| {
                (
                    core::mem::take(&mut o.signals),
                    core::mem::take(&mut o.effects),
                )
            })
            .unwrap_or_default()
    })
}

pub(crate) fn take_deferred_dispose(rt: &mut RuntimeState, eff_key: EffectKey) -> bool {
    if let Some(pos) = rt.dispose_after_run.iter().position(|k| *k == eff_key) {
        rt.dispose_after_run.swap_remove(pos);
        true
    } else {
        false
    }
}

fn defer_until_run_ends(rt: &mut RuntimeState, eff_key: EffectKey) {
    if !rt.dispose_after_run.contains(&eff_key) {
        rt.dispose_after_run.push(eff_key);
    }
}

fn running_effect_for_run_owner(rt: &RuntimeState, key: OwnerKey) -> Option<EffectKey> {
    rt.running.iter().copied().find(|eff| {
        rt.effects
            .get(*eff)
            .is_some_and(|slot| slot.run_owner == Some(key))
    })
}

pub(crate) fn remove_effect(rt: &mut RuntimeState, eff_key: EffectKey) -> Option<OwnerKey> {
    let eff = rt.effects.get_mut(eff_key)?;
    let run_owner = eff.run_owner.take();
    let mut deps = eff.deps.clone();
    for dep in &eff.prev_deps {
        if !deps.contains(dep) {
            deps.push(*dep);
        }
    }
    for dep in deps {
        if let Some(sig) = rt.signals.get_mut(dep)
            && let Some(pos) = sig.subscribers.iter().position(|k| *k == eff_key)
        {
            sig.subscribers.swap_remove(pos);
        }
    }
    rt.effects.remove(eff_key);
    rt.pending.retain(|p| *p != eff_key);
    rt.pending_set.remove(&eff_key);
    rt.running.remove(&eff_key);
    run_owner
}

fn remove_effects_and_signals(
    rt: &mut RuntimeState,
    signals: Vec<SignalKey>,
    effects: Vec<EffectKey>,
) -> Vec<OwnerKey> {
    for sig_key in signals {
        rt.signals.remove(sig_key);
    }
    let mut run_owners = Vec::new();
    for eff_key in effects {
        if rt.running.contains(&eff_key) {
            defer_until_run_ends(rt, eff_key);
            continue;
        }
        if let Some(run_owner) = remove_effect(rt, eff_key) {
            run_owners.push(run_owner);
        }
    }
    run_owners
}

fn collect_plain_subtree(rt: &RuntimeState, key: OwnerKey) -> Option<Vec<OwnerKey>> {
    let mut stack = vec![key];
    let mut owners = Vec::new();
    while let Some(current) = stack.pop() {
        if running_effect_for_run_owner(rt, current).is_some() {
            return None;
        }
        let Some(owner) = rt.owners.get(current) else {
            continue;
        };
        if !owner.cleanups.is_empty() || !owner.signals.is_empty() || !owner.effects.is_empty() {
            return None;
        }
        owners.push(current);
        for &child in &owner.children {
            stack.push(child);
        }
    }
    Some(owners)
}

pub(crate) fn try_dispose_plain_subtree(key: OwnerKey) -> bool {
    with_rt(|rt| {
        let Some(root) = rt.owners.get(key) else {
            return true;
        };
        let parent = root.parent.map(|parent| (parent, root.index_in_parent));

        if root.children.is_empty()
            && root.cleanups.is_empty()
            && root.signals.is_empty()
            && root.effects.is_empty()
        {
            if let Some((parent, index_in_parent)) = parent {
                detach_from_parent(rt, key, parent, index_in_parent);
            }
            rt.owners.remove(key);
            return true;
        }

        if !root.cleanups.is_empty() || !root.signals.is_empty() || !root.effects.is_empty() {
            return false;
        }

        let Some(owners) = collect_plain_subtree(rt, key) else {
            return false;
        };

        if let Some((parent, index_in_parent)) = parent {
            detach_from_parent(rt, key, parent, index_in_parent);
        }
        for owner in owners {
            rt.owners.remove(owner);
        }
        true
    })
}

pub(crate) fn detach_from_parent(
    rt: &mut RuntimeState,
    key: OwnerKey,
    parent: OwnerKey,
    index_in_parent: Option<usize>,
) {
    let mut moved_child = None;
    let mut reindex_children = None;

    {
        let Some(parent_slot) = rt.owners.get_mut(parent) else {
            return;
        };
        if let Some(index) = index_in_parent
            && parent_slot
                .children
                .get(index)
                .is_some_and(|&child| child == key)
        {
            let last = parent_slot.children.last().copied();
            parent_slot.children.swap_remove(index);
            if let Some(last) = last
                && last != key
            {
                moved_child = Some((last, index));
            }
        } else {
            parent_slot.children.retain(|&child| child != key);
            reindex_children = Some(parent_slot.children.clone());
        }
    }

    if let Some((child, index)) = moved_child
        && let Some(slot) = rt.owners.get_mut(child)
    {
        slot.index_in_parent = Some(index);
    }
    if let Some(children) = reindex_children {
        for (index, child) in children.into_iter().enumerate() {
            if let Some(slot) = rt.owners.get_mut(child) {
                slot.index_in_parent = Some(index);
            }
        }
    }
}

fn run_owner_cleanups(key: OwnerKey) {
    let cleanups = with_rt(|rt| {
        rt.owners
            .get_mut(key)
            .map(|o| core::mem::take(&mut o.cleanups))
            .unwrap_or_default()
    });
    if cleanups.is_empty() {
        return;
    }
    if std::thread::panicking() {
        let deferred = cleanups.len();
        with_rt(|rt| rt.deferred_cleanups.extend(cleanups));
        tracing::debug!(
            target: "sigil::reactive",
            deferred,
            "owner cleanups deferred until panic recovery"
        );
        return;
    }
    for cleanup in cleanups.into_iter().rev() {
        cleanup();
    }
}

fn defer_running_run_owner(key: OwnerKey) -> bool {
    with_rt(|rt| {
        let Some(eff_key) = running_effect_for_run_owner(rt, key) else {
            return false;
        };
        defer_until_run_ends(rt, eff_key);
        true
    })
}

pub(crate) fn dispose_owner(key: OwnerKey) {
    if defer_running_run_owner(key) {
        return;
    }
    if try_dispose_plain_subtree(key) {
        return;
    }

    let children = with_rt(|rt| {
        rt.owners
            .get(key)
            .map(|o| o.children.clone())
            .unwrap_or_default()
    });
    for child in children {
        dispose_owner(child);
    }

    run_owner_cleanups(key);

    let (signals, effects) = take_owner_resources(key);
    let orphan_run_owners = with_rt(|rt| remove_effects_and_signals(rt, signals, effects));
    for run_owner in orphan_run_owners {
        dispose_owner(run_owner);
    }

    with_rt(|rt| {
        let parent = rt
            .owners
            .get(key)
            .and_then(|o| o.parent.map(|parent| (parent, o.index_in_parent)));
        if let Some((parent, index_in_parent)) = parent {
            detach_from_parent(rt, key, parent, index_in_parent);
        }
        rt.owners.remove(key);
    });
}

#[cfg(test)]
#[path = "owner_dispose_tests.rs"]
mod tests;
