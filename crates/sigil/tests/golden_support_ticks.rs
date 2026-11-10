#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil::animation::SpringPreset;
use sigil::animation::{Animator, MotionPrefs, active_count, tick};
use sigil::prelude::*;
use sigil::reactive::{Owner, signal};

use sigil_kittest::{InteractionMode, SETTLE_SAFETY_CAP};

const DT_60HZ: f32 = 1.0 / 60.0;

fn drive_in_harness(mode: InteractionMode) {
    if let Err(err) = sigil_kittest::Harness::render(|| button("tick"))
        .stub()
        .mode(mode)
        .size([64, 64])
        .hit((10.0, 10.0))
        .image()
    {
        panic!("{err}");
    }
}

#[test]
fn after_frames_calls_tick_exactly_n_times() {
    Owner::scope("after_frames_calls_tick_exactly_n_times", || {
        MotionPrefs::set(false);
        let value = signal(0.0_f32);
        let mut anim = Animator::tween(value, 10_000, sigil::animation::Easing::Linear);
        anim.target(1.0);
        let before = value.peek();
        assert!(
            (before - 0.0).abs() < f32::EPSILON,
            "tween must not advance before any tick (got {before})",
        );

        drive_in_harness(InteractionMode::HoverAfterFrames {
            frames: 6,
            dt: DT_60HZ,
        });

        let after = value.peek();
        let expected = 6.0 * DT_60HZ / 10.0;
        let delta = (after - expected).abs();
        assert!(
            delta < 1e-4,
            "after 6 ticks expected ≈ {expected}, got {after} (delta {delta}). \
             Harness did not tick exactly 6 times.",
        );
        assert!(!anim.settled(), "tween should not have settled in 6 ticks");
    });
}

#[test]
fn after_frames_with_u32_max_stops_when_settled() {
    Owner::scope("after_frames_with_u32_max_stops_when_settled", || {
        MotionPrefs::set(false);
        let value = signal(0.0_f32);
        let mut anim = Animator::spring(value, SpringPreset::Stiff);
        anim.target(1.0);
        assert_eq!(active_count(), 1);

        drive_in_harness(InteractionMode::HoverAfterFrames {
            frames: u32::MAX,
            dt: DT_60HZ,
        });

        assert!(
            anim.settled(),
            "Stiff spring should settle well within the safety cap; \
             got settled={} value={}",
            anim.settled(),
            anim.value(),
        );
        assert_eq!(
            active_count(),
            0,
            "settled animators should be evicted from the active set",
        );
    });
}

#[test]
fn after_frames_safety_cap_panics_on_runaway() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Owner::scope("after_frames_safety_cap_panics_on_runaway", || {
            MotionPrefs::set(false);
            let value = signal(0.0_f32);
            let mut anim = Animator::tween(value, 60_000, sigil::animation::Easing::Linear);
            anim.target(1.0);
            assert_eq!(active_count(), 1);

            drive_in_harness(InteractionMode::HoverAfterFrames {
                frames: u32::MAX,
                dt: DT_60HZ,
            });
        });
    }));

    let err = result.expect_err("runaway animator must trip the safety cap");
    let msg = err
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| err.downcast_ref::<&'static str>().copied())
        .unwrap_or("");
    assert!(
        msg.contains("SETTLE_SAFETY_CAP") && msg.contains(&SETTLE_SAFETY_CAP.to_string()),
        "panic message must reference the safety cap; got {msg:?}",
    );

    for _ in 0..SETTLE_SAFETY_CAP {
        if active_count() == 0 {
            break;
        }
        tick(0.0);
        tick(120.0);
    }
}
