#![allow(clippy::unwrap_used, clippy::panic)]

use sigil_animation::SpringPreset;
use sigil_reactive::Runtime;
use sigil_style::{Px, StateSlot, StateStyle, StyleAtom, TransitionId};

use super::{ANIMATION_REGISTRY_MISS, install_animators, miss_counter};
use crate::registry::presets;
use crate::{AnimationConfig, MotionPrefs, active_count, tick};

const DT_60HZ: f32 = 1.0 / 60.0;

fn fresh_runtime<F: FnOnce()>(name: &'static str, body: F) {
    Runtime::scope(name, || {
        MotionPrefs::set(false);
        body();
    });
}

#[test]
fn unknown_transition_id_logs_warning() {
    fresh_runtime("w5_1_unknown_transition_id_logs_warning", || {
        // ComponentId namespace specific to this test so a parallel
        // test running the same scenario doesn't inflate our count.
        let component_id = 0xA111_BEEF_DEAD_0001_u64;
        let before = miss_counter::for_component(component_id);

        let atoms = [
            StyleAtom::Opacity(8_000),
            StyleAtom::Transition(TransitionId(0xFFFF)),
        ];
        let state_style = StateStyle::EMPTY.with_hovered(&[StyleAtom::Opacity(10_000)]);
        let attached = install_animators(&atoms, state_style, component_id);

        assert!(
            attached.is_empty(),
            "unknown TransitionId must fall back to no-animation",
        );
        assert_eq!(
            miss_counter::for_component(component_id),
            before + 1,
            "registry miss must bump the per-component counter",
        );
        // The crate-wide warning constant is documented public —
        // touch it so the symbol stays linked.
        assert_ne!(ANIMATION_REGISTRY_MISS, 0);
    });
}

#[test]
fn attached_animator_drives_signal() {
    fresh_runtime("w5_1_attached_animator_drives_signal", || {
        // Use a tween so the value progression is deterministic
        // across runs at fixed dt.
        let atoms = [
            StyleAtom::Opacity(5_000),
            StyleAtom::Transition(presets::DEFAULT),
        ];
        let state_style = StateStyle::EMPTY.with_hovered(&[StyleAtom::Opacity(10_000)]);

        let mut attached = install_animators(&atoms, state_style, 1);
        assert_eq!(attached.len(), 1, "one numeric delta → one animator");
        let initial = attached.iter().next().unwrap().signal.peek();
        assert!(
            (initial - 5_000.0).abs() < f32::EPSILON,
            "initial signal value comes from base atom (got {initial})",
        );

        attached.retarget_for_state(StateSlot::Hovered, &atoms);

        // Tick a few frames; expect the bound signal to move from
        // 5_000 toward 10_000.
        for _ in 0..6 {
            tick(DT_60HZ);
        }
        let mid = attached.iter().next().unwrap().signal.peek();
        assert!(
            mid > initial,
            "tween should advance toward the hover target (initial={initial}, mid={mid})",
        );
        assert!(
            mid < 10_000.0,
            "after 6 frames the tween should not yet have reached target (mid={mid})",
        );
    });
}

#[test]
fn state_slot_transition_retargets_animator() {
    fresh_runtime("w5_1_state_slot_transition_retargets_animator", || {
        let atoms = [
            StyleAtom::BorderRadius(Px(4)),
            StyleAtom::Transition(presets::DEFAULT),
        ];
        let state_style = StateStyle::EMPTY
            .with_hovered(&[StyleAtom::BorderRadius(Px(12))])
            .with_pressed(&[StyleAtom::BorderRadius(Px(2))]);

        let mut attached = install_animators(&atoms, state_style, 2);
        assert_eq!(attached.len(), 1);

        // Default → Hovered: target rises to 12.
        attached.retarget_for_state(StateSlot::Hovered, &atoms);
        // Run a few ticks but don't drive to settled — proves the
        // animator is moving without snapping.
        for _ in 0..4 {
            tick(DT_60HZ);
        }
        let after_hover = attached.iter().next().unwrap().signal.peek();
        assert!(
            after_hover > 4.0 && after_hover < 12.0,
            "after hover-target the signal interpolates upward \
             (got {after_hover})",
        );

        // Hovered → Pressed: target retargets to 2 *without* snapping.
        attached.retarget_for_state(StateSlot::Pressed, &atoms);
        for _ in 0..4 {
            tick(DT_60HZ);
        }
        let after_press = attached.iter().next().unwrap().signal.peek();
        // The signal MAY temporarily overshoot upward as the tween
        // re-arms; what matters is the *direction* of travel is
        // toward the new (lower) target.
        assert!(
            after_press < after_hover,
            "after pressed-target the signal must move *toward* the \
             new target (got after_hover={after_hover}, after_press={after_press})",
        );
    });
}

#[test]
fn settled_binding_no_longer_ticks() {
    fresh_runtime("w5_1_settled_binding_no_longer_ticks", || {
        let atoms = [
            StyleAtom::Opacity(2_000),
            StyleAtom::Transition(presets::DEFAULT),
        ];
        let state_style = StateStyle::EMPTY.with_hovered(&[StyleAtom::Opacity(10_000)]);
        let baseline = active_count();

        let mut attached = install_animators(&atoms, state_style, 3);
        assert_eq!(attached.len(), 1);
        attached.retarget_for_state(StateSlot::Hovered, &atoms);

        // Tween default = 200 ms. At 60 Hz that's ~12 ticks. Run
        // generously, then assert eviction.
        let mut settled = false;
        for _ in 0..120 {
            tick(DT_60HZ);
            if attached.iter().next().unwrap().animator.settled() {
                settled = true;
                break;
            }
        }
        assert!(settled, "tween default must settle within 2 s of ticks");

        // One more tick after settled removes from active set.
        tick(DT_60HZ);
        assert_eq!(
            active_count(),
            baseline,
            "settled animator must drop out of the active set",
        );
    });
}

#[test]
fn no_transition_atom_is_zero_overhead() {
    fresh_runtime("w5_1_no_transition_atom_is_zero_overhead", || {
        let atoms = [StyleAtom::Opacity(8_000)];
        let state_style = StateStyle::EMPTY.with_hovered(&[StyleAtom::Opacity(10_000)]);
        let baseline = active_count();

        let attached = install_animators(&atoms, state_style, 4);

        assert!(attached.is_empty(), "no Transition atom → no animators");
        assert_eq!(active_count(), baseline);
    });
}

#[test]
fn spring_preset_drives_animator_when_id_resolves() {
    fresh_runtime("w5_1_spring_preset_drives_animator", || {
        let atoms = [
            StyleAtom::Opacity(0),
            StyleAtom::Transition(presets::SMOOTH),
        ];
        let state_style = StateStyle::EMPTY.with_hovered(&[StyleAtom::Opacity(10_000)]);
        let attached = install_animators(&atoms, state_style, 5);
        assert_eq!(attached.len(), 1, "SMOOTH spring resolves → one animator");

        // The registry should have returned a Spring config; verify
        // by looking it up via the public config surface.
        let config = crate::config(presets::SMOOTH).expect("SMOOTH preset");
        assert!(matches!(
            config,
            AnimationConfig::Spring {
                preset: SpringPreset::Smooth
            }
        ));
        drop(attached);
    });
}
