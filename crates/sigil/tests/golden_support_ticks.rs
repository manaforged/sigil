//! Unit tests for the animated-golden harness's `*AfterFrames`
//! interaction modes (W5.0a).
//!
//! These tests do NOT render — they exercise the tick-driving logic in
//! [`golden_support::drive_state`] directly by reading the
//! `animation_runtime` active-set state and the bound `Signal<f32>`
//! before and after the harness drive.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_animation::SpringPreset;
use sigil_animation_runtime::{AnimationConfig, Animator, MotionPrefs, active_count, tick};
use sigil_reactive::{Runtime, signal};

mod golden_support;

use golden_support::{InteractionMode, SETTLE_SAFETY_CAP};

const DT_60HZ: f32 = 1.0 / 60.0;

/// Construct a minimal root node and a fresh `EventRouter` + `LayoutEngine`
/// so we can call the harness driver in isolation. The widget is a no-op
/// stack — the tests exercise the runtime tick loop, not rendering.
fn drive_in_harness(mode: InteractionMode) {
    // The harness's `drive_state` is private to the support module, so we
    // exercise it through the public surface: `render_widget_to_png`
    // returns `None` when no GPU adapter is available, but it still walks
    // `drive_state` end-to-end up to that point. We pick a viewport that
    // forces a layout pass and a hit-point inside the (1, 1) stack so
    // pointer events route.
    let _ = golden_support::render_widget_to_png(
        |cx| cx.add(button("tick")),
        mode,
        [64, 64],
        (10.0, 10.0),
    );
}

#[test]
fn after_frames_calls_tick_exactly_n_times() {
    Runtime::scope("after_frames_calls_tick_exactly_n_times", || {
        MotionPrefs::set(false);
        let value = signal(0.0_f32);
        let mut anim = Animator::new(
            value,
            // Long tween so 6 ticks are well short of settle.
            AnimationConfig::Tween {
                ms: 10_000,
                easing: sigil_animation::Easing::Linear,
            },
        );
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

        // Linear tween over 10 s, 6 ticks at 1/60 s = 0.1 s = 1% of the
        // way to 1.0. Expected ≈ 0.01; any non-trivial deviation means
        // the harness ticked the wrong number of times.
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
    Runtime::scope("after_frames_with_u32_max_stops_when_settled", || {
        MotionPrefs::set(false);
        let value = signal(0.0_f32);
        let mut anim = Animator::new(
            value,
            AnimationConfig::Spring {
                preset: SpringPreset::Stiff,
            },
        );
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
    // We construct a runaway by retargeting on every tick via a tween
    // whose duration is far longer than the safety cap's wall-time. At
    // dt=1/60s and the cap of 600 frames, total simulated time is 10 s;
    // a 60 s tween won't be near settled.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Runtime::scope("after_frames_safety_cap_panics_on_runaway", || {
            MotionPrefs::set(false);
            let value = signal(0.0_f32);
            let mut anim = Animator::new(
                value,
                AnimationConfig::Tween {
                    ms: 60_000,
                    easing: sigil_animation::Easing::Linear,
                },
            );
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

    // Drain the runaway animator so it doesn't leak into a sibling test
    // on this thread (thread-locals persist across `Runtime::scope`).
    for _ in 0..SETTLE_SAFETY_CAP {
        if active_count() == 0 {
            break;
        }
        tick(0.0); // no-op, just to remind future readers tick(0) is rejected
        // Force eviction by retargeting to current then ticking with a
        // large dt to finish the tween.
        tick(120.0);
    }
}
