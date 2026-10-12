#![allow(clippy::unwrap_used)]

use crate::animation::SpringPreset;
use crate::reactive::Runtime;
use crate::style::{Px, StateSlot, StateStyle, StyleAtom, TransitionId};

use super::{ANIMATION_REGISTRY_MISS, AttachedAnimators, install_animators, miss_counter};
use crate::animation_runtime::registry::presets;
use crate::animation_runtime::{AnimationConfig, MotionPrefs, active_count, tick};

fn fresh_runtime(name: &'static str, body: impl FnOnce()) {
    Runtime::scope(name, || {
        MotionPrefs::set(false);
        body();
    });
}

fn opacity_to_full(base: u16, transition: TransitionId) -> ([StyleAtom; 2], StateStyle) {
    (
        [StyleAtom::Opacity(base), StyleAtom::Transition(transition)],
        StateStyle::EMPTY.with_hovered(&[StyleAtom::Opacity(10_000)]),
    )
}

fn ticks(frames: usize) {
    for _ in 0..frames {
        tick(1.0 / 60.0);
    }
}

fn first_value(attached: &AttachedAnimators) -> f32 {
    attached.iter().next().unwrap().signal.peek()
}

#[test]
fn unknown_transition_id_logs_warning() {
    fresh_runtime("unknown_transition_id_logs_warning", || {
        let component_id = 0xA111_BEEF_DEAD_0001_u64;
        let before = miss_counter::for_component(component_id);
        let (atoms, state_style) = opacity_to_full(8_000, TransitionId(0xFFFF));
        assert!(install_animators(&atoms, state_style, component_id).is_empty());
        assert_eq!(miss_counter::for_component(component_id), before + 1);
        assert_ne!(ANIMATION_REGISTRY_MISS, 0);
    });
}

#[test]
fn attached_animator_drives_signal() {
    fresh_runtime("attached_animator_drives_signal", || {
        let (atoms, state_style) = opacity_to_full(5_000, presets::DEFAULT);
        let mut attached = install_animators(&atoms, state_style, 1);
        assert_eq!(attached.len(), 1);
        let initial = first_value(&attached);
        assert!(
            (initial - 5_000.0).abs() < f32::EPSILON,
            "initial={initial}"
        );
        attached.retarget_for_state(StateSlot::Hovered, &atoms);
        ticks(6);
        let mid = first_value(&attached);
        assert!(
            mid > initial && mid < 10_000.0,
            "initial={initial}, mid={mid}"
        );
    });
}

#[test]
fn state_slot_transition_retargets_animator() {
    fresh_runtime("state_slot_transition_retargets_animator", || {
        let atoms = [
            StyleAtom::BorderRadius(Px(4)),
            StyleAtom::Transition(presets::DEFAULT),
        ];
        let state_style = StateStyle::EMPTY
            .with_hovered(&[StyleAtom::BorderRadius(Px(12))])
            .with_pressed(&[StyleAtom::BorderRadius(Px(2))]);
        let mut attached = install_animators(&atoms, state_style, 2);
        assert_eq!(attached.len(), 1);
        attached.retarget_for_state(StateSlot::Hovered, &atoms);
        ticks(4);
        let after_hover = first_value(&attached);
        assert!(after_hover > 4.0 && after_hover < 12.0, "{after_hover}");
        attached.retarget_for_state(StateSlot::Pressed, &atoms);
        ticks(4);
        let after_press = first_value(&attached);
        assert!(after_press < after_hover, "{after_hover} -> {after_press}");
    });
}

#[test]
fn settled_binding_no_longer_ticks() {
    fresh_runtime("settled_binding_no_longer_ticks", || {
        let (atoms, state_style) = opacity_to_full(2_000, presets::DEFAULT);
        let baseline = active_count();
        let mut attached = install_animators(&atoms, state_style, 3);
        assert_eq!(attached.len(), 1);
        attached.retarget_for_state(StateSlot::Hovered, &atoms);
        let settled = (0..120).any(|_| {
            tick(1.0 / 60.0);
            attached.iter().next().unwrap().animator.settled()
        });
        assert!(settled, "tween default must settle within 2 s of ticks");
        tick(1.0 / 60.0);
        assert_eq!(active_count(), baseline);
    });
}

#[test]
fn no_transition_atom_is_zero_overhead() {
    fresh_runtime("no_transition_atom_is_zero_overhead", || {
        let state_style = StateStyle::EMPTY.with_hovered(&[StyleAtom::Opacity(10_000)]);
        let baseline = active_count();
        assert!(install_animators(&[StyleAtom::Opacity(8_000)], state_style, 4).is_empty());
        assert_eq!(active_count(), baseline);
    });
}

#[test]
fn spring_preset_drives_animator_when_id_resolves() {
    fresh_runtime("spring_preset_drives_animator", || {
        let (atoms, state_style) = opacity_to_full(0, presets::SMOOTH);
        assert_eq!(install_animators(&atoms, state_style, 5).len(), 1);
        assert!(matches!(
            crate::animation_runtime::config(presets::SMOOTH),
            Some(AnimationConfig::Spring {
                preset: SpringPreset::Smooth
            })
        ));
    });
}
