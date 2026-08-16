//! Per-thread active-animator set + the [`tick`] entry point.
//!
//! Animators store only an ID and a [`Signal<f32>`] handle; the heavy
//! state (the wrapped spring or tween) lives here in a thread-local
//! `Vec` so iteration during [`tick`] needs no double-indirection. IDs
//! are allocated monotonically per-thread.

use core::cell::{Cell, RefCell};

use sigil_animation::{Easing, SpringPreset};
use sigil_reactive::Signal;

use crate::animator::{Motion, SpringMotion, TweenMotion};

/// Opaque per-thread handle. Animators carry one; the runtime uses it
/// to locate the entry on retarget / settle queries / deregistration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnimatorId(u32);

struct Entry {
    id: AnimatorId,
    signal: Signal<f32>,
    motion: Box<dyn Motion>,
}

thread_local! {
    static ENTRIES: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
    static NEXT_ID: Cell<u32> = const { Cell::new(1) };
}

fn next_id() -> AnimatorId {
    NEXT_ID.with(|cell| {
        let id = cell.get();
        cell.set(id.wrapping_add(1).max(1));
        AnimatorId(id)
    })
}

pub(crate) fn register_spring(signal: Signal<f32>, preset: SpringPreset) -> AnimatorId {
    let id = next_id();
    let motion = Box::new(SpringMotion::from_preset(signal.peek(), preset));
    ENTRIES.with(|cell| {
        cell.borrow_mut().push(Entry { id, signal, motion });
    });
    id
}

pub(crate) fn register_tween(signal: Signal<f32>, ms: u16, easing: Easing) -> AnimatorId {
    let id = next_id();
    let motion = Box::new(TweenMotion::new(signal.peek(), ms, easing));
    ENTRIES.with(|cell| {
        cell.borrow_mut().push(Entry { id, signal, motion });
    });
    id
}

pub(crate) fn deregister(id: AnimatorId) {
    ENTRIES.with(|cell| {
        let mut entries = cell.borrow_mut();
        if let Some(pos) = entries.iter().position(|entry| entry.id == id) {
            entries.swap_remove(pos);
        }
    });
}

pub(crate) fn retarget(id: AnimatorId, value: f32) {
    ENTRIES.with(|cell| {
        let mut entries = cell.borrow_mut();
        if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
            entry.motion.retarget(value);
        }
    });
}

pub(crate) fn is_settled(id: AnimatorId) -> bool {
    ENTRIES.with(|cell| {
        let entries = cell.borrow();
        entries
            .iter()
            .find(|entry| entry.id == id)
            .is_none_or(|entry| entry.motion.is_settled())
    })
}

/// Snapshot the current driven value for the given animator. Returns
/// `None` when no entry matches (reduced-motion / already dropped).
pub(crate) fn peek_value(id: AnimatorId) -> Option<f32> {
    ENTRIES.with(|cell| {
        cell.borrow()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.motion.value())
    })
}

/// Advance every active animator by `dt` seconds.
///
/// Writes new values into the bound signals. Settled entries stay registered
/// until their [`Animator`](crate::Animator) handle is dropped so a later
/// [`Animator::target`](crate::Animator::target) can re-arm the same motion;
/// [`active_count`] excludes those idle entries. Idempotent on empty: zero
/// cost when nothing is in flight.
///
/// Apps drive this once per animation frame during the Effect phase, before
/// layout, so geometry and paint observe the same value in that frame.
pub fn tick(dt: f32) {
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }
    // Drain into a local Vec so signal writes can re-enter the runtime
    // without holding the borrow on ENTRIES.
    let drained: Vec<Entry> = ENTRIES.with(|cell| core::mem::take(&mut *cell.borrow_mut()));
    let mut kept: Vec<Entry> = Vec::with_capacity(drained.len());
    for mut entry in drained {
        if !entry.motion.is_settled() {
            let _ = entry.motion.tick(dt);
            entry.signal.set(entry.motion.value());
        }
        kept.push(entry);
    }
    ENTRIES.with(|cell| {
        let mut entries = cell.borrow_mut();
        if entries.is_empty() {
            *entries = kept;
        } else {
            // Animators registered mid-tick (e.g. by an effect that
            // fired on a signal write inside this tick pass) appended
            // to ENTRIES while we were draining; preserve them.
            entries.splice(0..0, kept);
        }
    });
}

/// Number of currently moving animators on this thread.
///
/// Public test seam used by the visual-golden harness and by this
/// crate's unit tests. Production frame loops also use it to stop requesting
/// animation frames once every registered handle is idle.
#[must_use]
pub fn active_count() -> usize {
    ENTRIES.with(|cell| {
        cell.borrow()
            .iter()
            .filter(|entry| !entry.motion.is_settled())
            .count()
    })
}

#[cfg(test)]
mod tests {
    use sigil_animation::SpringPreset;
    use sigil_reactive::{Runtime, signal};

    use super::{active_count, tick};
    use crate::animator::Animator;
    use crate::config::AnimationConfig;
    use crate::motion::MotionPrefs;

    const DT_60HZ: f32 = 1.0 / 60.0;

    #[test]
    fn dropping_animator_deregisters_from_runtime() {
        Runtime::scope("dropping_animator_deregisters_from_runtime", || {
            MotionPrefs::set(false);
            let value = signal(0.0_f32);
            let baseline = active_count();

            {
                let mut animator = Animator::new(
                    value,
                    AnimationConfig::Spring {
                        preset: SpringPreset::Smooth,
                    },
                );
                animator.target(1.0);
                assert_eq!(active_count(), baseline + 1);
            }
            assert_eq!(
                active_count(),
                baseline,
                "active count did not return to baseline after drop"
            );
        });
    }

    #[test]
    fn reduced_motion_skips_animator_registration() {
        Runtime::scope("reduced_motion_skips_animator_registration", || {
            MotionPrefs::set(true);
            assert!(MotionPrefs::reduced());

            let value = signal(0.0_f32);
            let baseline = active_count();

            let mut animator = Animator::new(
                value,
                AnimationConfig::Spring {
                    preset: SpringPreset::Smooth,
                },
            );
            // No registration → no entries.
            assert_eq!(active_count(), baseline);
            assert!(animator.settled());

            // Targeting jumps the signal instantly; no ticks needed.
            animator.target(1.0);
            assert!((value.peek() - 1.0).abs() < f32::EPSILON);
            assert_eq!(active_count(), baseline);

            // Restore for sibling tests.
            MotionPrefs::set(false);
        });
    }

    #[test]
    fn settled_animator_becomes_idle_and_can_retarget() {
        Runtime::scope("settled_animator_becomes_idle_and_can_retarget", || {
            MotionPrefs::set(false);
            let value = signal(0.0_f32);
            let baseline = active_count();
            let mut animator = Animator::new(
                value,
                AnimationConfig::Spring {
                    preset: SpringPreset::Stiff,
                },
            );
            animator.target(1.0);

            assert_eq!(active_count(), baseline + 1);

            // Tick until the spring reports settled. Stiff preset is fast;
            // 60 frames is more than enough.
            for _ in 0..60 {
                tick(DT_60HZ);
                if animator.settled() {
                    break;
                }
            }
            assert!(animator.settled());

            assert_eq!(
                active_count(),
                baseline,
                "settled animator must stop consuming animation frames"
            );

            animator.target(0.25);
            assert_eq!(
                active_count(),
                baseline + 1,
                "target must re-arm a settled animator"
            );
            tick(DT_60HZ);
            assert!(
                animator.value() < 1.0,
                "re-armed animator must move toward its new target"
            );

            drop(animator);
            assert_eq!(active_count(), baseline);
        });
    }
}
