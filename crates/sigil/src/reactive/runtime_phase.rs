#[cfg(any(test, feature = "desktop"))]
use core::panic::Location;

#[cfg(host)]
use crate::reactive::owner_dispose::dispose_owner;
use crate::reactive::phase::DrawPhase;
use crate::reactive::runtime::{Runtime, RuntimeState, with_rt};

impl Runtime {
    kittest_pub! {
        /// Resets the runtime after a caught panic.
        #[cfg(host)]
        pub fn recover_from_panic() {
            let run_owners = with_rt(|rt| {
                let deferred = core::mem::take(&mut rt.dispose_after_run);
                let mut run_owners = Vec::new();
                for eff_key in deferred {
                    if rt.running.contains(&eff_key) {
                        rt.dispose_after_run.push(eff_key);
                        continue;
                    }
                    if let Some(run_owner) = rt.remove_effect(eff_key) {
                        run_owners.push(run_owner);
                    }
                }
                run_owners
            });
            for run_owner in run_owners {
                dispose_owner(run_owner);
            }
            let deferred_cleanups = with_rt(|rt| {
                rt.phase = DrawPhase::Idle;
                rt.running.clear();
                rt.panic_origin = None;
                clear_dead_current(rt);
                core::mem::take(&mut rt.deferred_cleanups)
            });
            drop(with_rt(|rt| core::mem::take(&mut rt.after_run)));
            run_deferred_cleanups(deferred_cleanups);
        }
    }

    pub(crate) fn recover_after_swallowed_panic() {
        let deferred_cleanups = with_rt(|rt| {
            rt.panic_origin = None;
            clear_dead_current(rt);
            core::mem::take(&mut rt.deferred_cleanups)
        });
        run_deferred_cleanups(deferred_cleanups);
    }

    /// Returns the name and location of the effect that panicked.
    #[cfg(any(test, feature = "desktop"))]
    #[must_use]
    pub(crate) fn panic_origin_info() -> Option<(&'static str, &'static Location<'static>)> {
        with_rt(|rt| rt.panic_origin.map(|(_, name, location)| (name, location)))
    }

    kittest_pub! {
        /// Enters the effect phase.
        pub fn begin_effect() {
            advance("begin_effect", DrawPhase::Idle, DrawPhase::Effect);
            with_rt(|rt| rt.frame_dirty = false);
        }
    }

    kittest_pub! {
        /// Enters the layout phase.
        pub fn begin_layout() {
            advance("begin_layout", DrawPhase::Effect, DrawPhase::Layout);
        }
    }

    kittest_pub! {
        /// Enters the paint phase.
        pub fn begin_paint() {
            advance("begin_paint", DrawPhase::Layout, DrawPhase::Paint);
            with_rt(|rt| rt.paints += 1);
        }
    }

    pub(crate) fn layout_paints() -> Option<u64> {
        with_rt(|rt| (rt.phase == DrawPhase::Layout).then_some(rt.paints))
    }

    kittest_pub! {
        /// Ends the current frame.
        pub fn end_frame() {
            advance("end_frame", DrawPhase::Paint, DrawPhase::Idle);
        }
    }

    kittest_pub! {
        /// Returns whether something asked for another frame.
        #[must_use]
        pub fn frame_dirty() -> bool {
            with_rt(|rt| rt.frame_dirty)
        }
    }

    kittest_pub! {
        /// Asks for another frame.
        pub fn mark_frame_dirty() {
            with_rt(|rt| rt.frame_dirty = true);
        }
    }
}

fn advance(api: &'static str, from: DrawPhase, to: DrawPhase) {
    with_rt(|rt| {
        debug_assert_eq!(
            rt.phase, from,
            "Runtime::{api} called from phase {:?}, expected {from:?}",
            rt.phase,
        );
        rt.phase = to;
    });
}

fn clear_dead_current(rt: &mut RuntimeState) {
    if rt.current_owner.is_some_and(|key| !rt.owners.contains(key)) {
        rt.current_owner = None;
    }
    if rt
        .lazy_effect_owner
        .is_some_and(|key| !rt.effects.contains(key))
    {
        rt.lazy_effect_owner = None;
    }
    if rt
        .current_effect
        .is_some_and(|key| !rt.effects.contains(key))
    {
        rt.current_effect = None;
    }
}

fn run_deferred_cleanups(cleanups: Vec<Box<dyn FnOnce()>>) {
    for cleanup in cleanups.into_iter().rev() {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(cleanup)).is_err() {
            tracing::error!(
                target: "sigil::reactive",
                "deferred cleanup panicked during panic recovery"
            );
        }
    }
}
