use core::panic::Location;

use smallvec::SmallVec;

use crate::effect_run::run_effect;
use crate::runtime::{EffectKey, EffectSlot, with_rt};

#[track_caller]
pub fn effect<F: FnMut() + 'static>(f: F) {
    effect_named_impl("<unnamed>", f, Location::caller());
}

#[track_caller]
pub fn effect_named<F: FnMut() + 'static>(name: &'static str, f: F) {
    effect_named_impl(name, f, Location::caller());
}

fn effect_named_impl<F: FnMut() + 'static>(
    name: &'static str,
    f: F,
    location: &'static Location<'static>,
) {
    let key = with_rt(|rt| {
        let owner = rt
            .current_owner
            .unwrap_or_else(|| crate::diagnostic::no_scope("effect()", location));
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
