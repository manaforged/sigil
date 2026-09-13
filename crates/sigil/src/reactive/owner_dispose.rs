use crate::reactive::runtime::{
    EffectKey, EntityKey, ObserverKey, OwnerKey, RuntimeState, SignalKey, with_rt,
};

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

fn take_effect_run_owners(effects: &[EffectKey]) -> Vec<OwnerKey> {
    with_rt(|rt| {
        effects
            .iter()
            .filter_map(|&k| rt.effects.get_mut(k).and_then(|e| e.run_owner.take()))
            .collect()
    })
}

fn remove_effects_and_signals(
    rt: &mut RuntimeState,
    signals: Vec<SignalKey>,
    effects: Vec<EffectKey>,
) {
    for sig_key in signals {
        rt.signals.remove(sig_key);
    }
    for eff_key in effects {
        let deps = rt
            .effects
            .get(eff_key)
            .map(|e| {
                let mut deps = e.deps.clone();
                for dep in &e.prev_deps {
                    if !deps.contains(dep) {
                        deps.push(*dep);
                    }
                }
                deps
            })
            .unwrap_or_default();
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
    }
}

fn evict_owner_entities(rt: &mut RuntimeState, entities: Vec<EntityKey>) {
    for entity_key in entities {
        let observers: Vec<ObserverKey> = rt
            .entities
            .get(entity_key)
            .map(|slot| {
                slot.subscribers
                    .iter()
                    .copied()
                    .chain(slot.event_subs.iter().map(|(_, k)| *k))
                    .collect()
            })
            .unwrap_or_default();
        for obs_key in observers {
            rt.observers.remove(obs_key);
            rt.pending_observers.retain(|p| p.observer != obs_key);
        }
        rt.globals.retain(|(_, k)| *k != entity_key);
        rt.entities.remove(entity_key);
    }
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
            && root.entities.is_empty()
        {
            if let Some((parent, index_in_parent)) = parent {
                detach_from_parent(rt, key, parent, index_in_parent);
            }
            rt.owners.remove(key);
            return true;
        }

        if !root.cleanups.is_empty()
            || !root.signals.is_empty()
            || !root.effects.is_empty()
            || !root.entities.is_empty()
        {
            return false;
        }

        let mut stack = vec![key];
        let mut owners = Vec::new();

        while let Some(current) = stack.pop() {
            let Some(owner) = rt.owners.get(current) else {
                continue;
            };
            if !owner.cleanups.is_empty()
                || !owner.signals.is_empty()
                || !owner.effects.is_empty()
                || !owner.entities.is_empty()
            {
                return false;
            }
            owners.push(current);
            for &child in &owner.children {
                stack.push(child);
            }
        }

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

pub(crate) fn dispose_owner(key: OwnerKey) {
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

    let cleanups = with_rt(|rt| {
        rt.owners
            .get_mut(key)
            .map(|o| core::mem::take(&mut o.cleanups))
            .unwrap_or_default()
    });
    for cleanup in cleanups.into_iter().rev() {
        cleanup();
    }

    let (signals, effects) = take_owner_resources(key);
    for run_owner in take_effect_run_owners(&effects) {
        dispose_owner(run_owner);
    }
    with_rt(|rt| remove_effects_and_signals(rt, signals, effects));

    let entities = with_rt(|rt| {
        rt.owners
            .get_mut(key)
            .map(|o| core::mem::take(&mut o.entities))
            .unwrap_or_default()
    });
    with_rt(|rt| evict_owner_entities(rt, entities));

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
