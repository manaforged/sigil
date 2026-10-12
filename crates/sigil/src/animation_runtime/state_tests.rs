use crate::animation::SpringPreset;
use crate::reactive::{Runtime, signal};

use super::{active_count, tick};
use crate::animation_runtime::animator::Animator;
use crate::animation_runtime::config::AnimationConfig;
use crate::animation_runtime::motion::MotionPrefs;

fn spring(preset: SpringPreset) -> AnimationConfig {
    AnimationConfig::Spring { preset }
}

#[test]
fn dropping_animator_deregisters_from_runtime() {
    Runtime::scope("dropping_animator_deregisters_from_runtime", || {
        MotionPrefs::set(false);
        let baseline = active_count();
        let mut animator = Animator::new(signal(0.0_f32), spring(SpringPreset::Smooth));
        animator.target(1.0);
        assert_eq!(active_count(), baseline + 1);
        drop(animator);
        assert_eq!(active_count(), baseline);
    });
}

#[test]
fn reduced_motion_skips_animator_registration() {
    Runtime::scope("reduced_motion_skips_animator_registration", || {
        MotionPrefs::set(true);
        assert!(MotionPrefs::reduced());
        let value = signal(0.0_f32);
        let baseline = active_count();
        let mut animator = Animator::new(value, spring(SpringPreset::Smooth));
        assert_eq!(active_count(), baseline);
        assert!(animator.settled());
        animator.target(1.0);
        assert!((value.peek() - 1.0).abs() < f32::EPSILON);
        assert_eq!(active_count(), baseline);
        MotionPrefs::set(false);
    });
}

#[test]
fn settled_animator_becomes_idle_and_can_retarget() {
    Runtime::scope("settled_animator_becomes_idle_and_can_retarget", || {
        MotionPrefs::set(false);
        let baseline = active_count();
        let mut animator = Animator::new(signal(0.0_f32), spring(SpringPreset::Stiff));
        animator.target(1.0);
        assert_eq!(active_count(), baseline + 1);
        for _ in 0..60 {
            tick(1.0 / 60.0);
            if animator.settled() {
                break;
            }
        }
        assert!(animator.settled());
        assert_eq!(active_count(), baseline, "settled animator stopped ticking");
        animator.target(0.25);
        assert_eq!(
            active_count(),
            baseline + 1,
            "target re-arms a settled animator"
        );
        tick(1.0 / 60.0);
        assert!(animator.value() < 1.0);
        drop(animator);
        assert_eq!(active_count(), baseline);
    });
}
