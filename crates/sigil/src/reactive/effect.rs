use core::panic::Location;

use smallvec::SmallVec;

use crate::reactive::diagnostic::{Cycle, no_scope, reactive_cycle};
use crate::reactive::effect_run::run_effect;
use crate::reactive::runtime::{EffectKey, EffectSlot, OwnerKey, with_rt};

const DRAIN_BUDGET: usize = 10_000;

#[track_caller]
pub fn effect<F: FnMut() + 'static>(f: F) {
    effect_named("<unnamed>", f);
}

#[track_caller]
pub fn effect_named<F: FnMut() + 'static>(name: &'static str, f: F) {
    let location = Location::caller();
    let key = with_rt(|rt| {
        let owner = rt
            .current_owner
            .unwrap_or_else(|| no_scope("effect()", location));
        assert!(
            rt.owners.contains(owner),
            "effect() with a disposed current owner (at {location})"
        );
        let key = rt.effects.insert(EffectSlot {
            owner,
            run_owner: None,
            f: Some(Box::new(f)),
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

pub fn untrack<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(Option<EffectKey>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let prev = self.0.take();
            with_rt(|rt| rt.current_effect = prev);
        }
    }
    let _restore = Restore(with_rt(|rt| rt.current_effect.take()));
    f()
}

pub fn defer(f: impl FnOnce() + 'static) {
    let f: Box<dyn FnOnce()> = Box::new(f);
    with_rt(|rt| {
        let key = rt.current_effect;
        rt.after_run.push((key, f));
    });
}

pub(crate) fn run_deferred(key: EffectKey) {
    let queued = with_rt(|rt| {
        let start = rt
            .after_run
            .iter()
            .rposition(|(owner, _)| *owner != Some(key))
            .map_or(0, |index| index + 1);
        rt.after_run.split_off(start)
    });
    for (_, f) in queued {
        f();
    }
}

pub(crate) fn run_dispatch_deferred() {
    let mut runs = 0;
    loop {
        let queued = with_rt(|rt| {
            if rt.current_effect.is_some() {
                return Vec::new();
            }
            let (queued, kept) = core::mem::take(&mut rt.after_run)
                .into_iter()
                .partition(|(owner, _)| owner.is_none());
            rt.after_run = kept;
            queued
        });
        if queued.is_empty() {
            return;
        }
        if runs >= DRAIN_BUDGET {
            with_rt(|rt| rt.after_run.extend(queued));
            reactive_cycle(Cycle::Budget(DRAIN_BUDGET));
            return;
        }
        runs += queued.len();
        crate::reactive::batch(|| {
            for (_, f) in queued {
                f();
            }
        });
    }
}

pub(crate) fn current_effect_owner() -> Option<OwnerKey> {
    with_rt(|rt| {
        rt.current_effect
            .and_then(|key| rt.effects.get(key).map(|slot| slot.owner))
    })
}

pub(crate) fn schedule_effects(effects: SmallVec<[EffectKey; 4]>) {
    let to_run = with_rt(|rt| {
        if rt.batch_depth > 0 {
            for e in effects {
                rt.enqueue(e);
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

pub(crate) fn drain_pending() {
    for _ in 0..DRAIN_BUDGET {
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
    if with_rt(|rt| !rt.pending.is_empty()) {
        reactive_cycle(Cycle::Budget(DRAIN_BUDGET));
    }
}
