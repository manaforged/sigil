use crate::animation::SpringPreset;
use crate::reactive::{Runtime, signal};

use super::{active_count, tick};
use crate::animation_runtime::animator::Animator;
use crate::animation_runtime::config::AnimationConfig;
use crate::animation_runtime::motion::MotionPrefs;

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
