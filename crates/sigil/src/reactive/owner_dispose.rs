use crate::reactive::arena::Arena;
use crate::reactive::runtime::{
    EffectKey, OwnerKey, OwnerSlot, RuntimeState, SignalKey, SignalSlot, with_rt,
};

pub(crate) fn unsubscribe(signals: &mut Arena<SignalSlot>, dep: SignalKey, effect: EffectKey) {
    if let Some(sig) = signals.get_mut(dep)
        && let Some(pos) = sig.subscribers.iter().position(|k| *k == effect)
    {
        sig.subscribers.swap_remove(pos);
    }
}

fn has_resources(owner: &OwnerSlot) -> bool {
    !owner.cleanups.is_empty() || !owner.signals.is_empty() || !owner.effects.is_empty()
}

impl RuntimeState {
    pub(crate) fn take_deferred_dispose(&mut self, eff_key: EffectKey) -> bool {
        if let Some(pos) = self.dispose_after_run.iter().position(|k| *k == eff_key) {
            self.dispose_after_run.swap_remove(pos);
            true
        } else {
            false
        }
    }

    fn defer_until_run_ends(&mut self, eff_key: EffectKey) {
        if !self.dispose_after_run.contains(&eff_key) {
            self.dispose_after_run.push(eff_key);
        }
    }

    fn running_effect_for_run_owner(&self, key: OwnerKey) -> Option<EffectKey> {
        self.running.iter().copied().find(|eff| {
            self.effects
                .get(*eff)
                .is_some_and(|slot| slot.run_owner == Some(key))
        })
    }

    pub(crate) fn remove_effect(&mut self, eff_key: EffectKey) -> Option<OwnerKey> {
        let eff = self.effects.get_mut(eff_key)?;
        let run_owner = eff.run_owner.take();
        for &dep in eff.deps.iter().chain(&eff.prev_deps) {
            unsubscribe(&mut self.signals, dep, eff_key);
        }
        self.effects.remove(eff_key);
        self.pending.retain(|p| *p != eff_key);
        self.pending_set.remove(&eff_key);
        self.running.remove(&eff_key);
        run_owner
    }

    fn remove_owner_resources(&mut self, key: OwnerKey) -> Vec<OwnerKey> {
        let Some(owner) = self.owners.get_mut(key) else {
            return Vec::new();
        };
        let signals = core::mem::take(&mut owner.signals);
        let effects = core::mem::take(&mut owner.effects);
        for sig_key in signals {
            self.signals.remove(sig_key);
        }
        let mut run_owners = Vec::new();
        for eff_key in effects {
            if self.running.contains(&eff_key) {
                self.defer_until_run_ends(eff_key);
                continue;
            }
            if let Some(run_owner) = self.remove_effect(eff_key) {
                run_owners.push(run_owner);
            }
        }
        run_owners
    }

    fn collect_plain_subtree(&self, key: OwnerKey) -> Option<Vec<OwnerKey>> {
        let mut stack = vec![key];
        let mut owners = Vec::new();
        while let Some(current) = stack.pop() {
            if self.running_effect_for_run_owner(current).is_some() {
                return None;
            }
            let Some(owner) = self.owners.get(current) else {
                continue;
            };
            if has_resources(owner) {
                return None;
            }
            owners.push(current);
            for &child in &owner.children {
                stack.push(child);
            }
        }
        Some(owners)
    }

    fn detach(&mut self, key: OwnerKey) {
        let Some((parent, index)) = self
            .owners
            .get(key)
            .and_then(|o| o.parent.map(|parent| (parent, o.index_in_parent)))
        else {
            return;
        };
        let Some(siblings) = self.owners.get_mut(parent).map(|p| &mut p.children) else {
            return;
        };
        if let Some(index) = index
            && siblings.get(index) == Some(&key)
        {
            siblings.swap_remove(index);
            if let Some(&moved) = siblings.get(index)
                && let Some(slot) = self.owners.get_mut(moved)
            {
                slot.index_in_parent = Some(index);
            }
            return;
        }
        siblings.retain(|&child| child != key);
        for (position, child) in siblings.clone().into_iter().enumerate() {
            if let Some(slot) = self.owners.get_mut(child) {
                slot.index_in_parent = Some(position);
            }
        }
    }
}

pub(crate) fn try_dispose_plain_subtree(key: OwnerKey) -> bool {
    with_rt(|rt| {
        let Some(root) = rt.owners.get(key) else {
            return true;
        };
        if has_resources(root) {
            return false;
        }
        if root.children.is_empty() {
            rt.detach(key);
            rt.owners.remove(key);
            return true;
        }
        let Some(owners) = rt.collect_plain_subtree(key) else {
            return false;
        };
        rt.detach(key);
        for owner in owners {
            rt.owners.remove(owner);
        }
        true
    })
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
        let Some(eff_key) = rt.running_effect_for_run_owner(key) else {
            return false;
        };
        rt.defer_until_run_ends(eff_key);
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

    let orphan_run_owners = with_rt(|rt| rt.remove_owner_resources(key));
    for run_owner in orphan_run_owners {
        dispose_owner(run_owner);
    }
    with_rt(|rt| {
        rt.detach(key);
        rt.owners.remove(key);
    });
}

#[cfg(test)]
#[path = "owner_dispose_tests.rs"]
mod tests;
