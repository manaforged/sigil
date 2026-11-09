#![allow(
    clippy::suboptimal_flops,
    reason = "the expected values spell out the curve formula"
)]

use std::f32::consts::{FRAC_PI_2, PI};
use std::time::Duration;

use sigil::animation::{Easing, Keyframes, Transition};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::style::Transform2D;
use sigil_kittest::Harness;

use super::{BLACK, WHITE, mount, near, near_px, run, seen, square, start, transitioned};

fn opacity_at(node: NodeBuilder, seconds: f32) -> f32 {
    let flags = [signal(false), signal(false)];
    let mut ui = mount(transitioned(flags, node.opacity(0.2)));
    start(&mut ui, flags[0]);
    run(&mut ui, seconds);
    seen(&mut ui).opacity
}

#[test]
fn tailwind_transition_is_150ms_on_the_default_curve() {
    Owner::scope("motion-tailwind-transition", || {
        let curve = 0.2 + 0.8 * 0.775_561;
        near(
            opacity_at(square().transition(Transition::new()), 0.075),
            curve,
            "transition at 75ms",
        );
        near(
            opacity_at(square().transition(Transition::new()), 0.15),
            1.0,
            "transition at 150ms",
        );
        near(
            opacity_at(square().transition(Transition::opacity()), 0.075),
            curve,
            "transition-opacity",
        );
        let slow = square().transition(Transition::new().duration(Duration::from_millis(300)));
        near(opacity_at(slow, 0.15), curve, "duration-300 at 150ms");
    });
}

#[test]
fn tailwind_ease_classes_follow_their_curves() {
    Owner::scope("motion-tailwind-ease", || {
        let at =
            |easing: Easing| opacity_at(square().transition(Transition::new().ease(easing)), 0.075);
        near(at(Easing::Linear), 0.6, "ease-linear");
        near(at(Easing::IN), 0.2 + 0.8 * 0.324_815, "ease-in");
        near(at(Easing::OUT), 0.2 + 0.8 * 0.839_245, "ease-out");
        near(at(Easing::IN_OUT), 0.2 + 0.8 * 0.775_561, "ease-in-out");
    });
}

fn colors_transition_animates_the_background() {
    let flag = signal(false);
    let colors = square()
        .bg(BLACK)
        .transition(Transition::colors())
        .paint_when(flag, |state| state.bg(WHITE));
    let mut ui = mount(colors);
    start(&mut ui, flag);
    run(&mut ui, 0.075);
    let mid = seen(&mut ui).color.r;
    assert!(
        mid > 0 && mid < 255,
        "transition-colors animates the background, got {mid}"
    );
}

fn transform_transition_animates_translate() {
    let flag = signal(false);
    let moved = Transform2D::IDENTITY.translate(100.0, 0.0);
    let transform = square()
        .transition(Transition::transform().ease(Easing::Linear))
        .paint_when(flag, move |state| state.transform(moved));
    let mut ui = mount(transform);
    start(&mut ui, flag);
    run(&mut ui, 0.075);
    near_px(
        seen(&mut ui).translate_x(),
        50.0,
        "transition-transform animates translate",
    );
}

fn opacity_transition_skips_colors() {
    let flag = signal(false);
    let opacity_only = square()
        .bg(BLACK)
        .transition(Transition::opacity())
        .paint_when(flag, |state| state.bg(WHITE));
    let mut ui = mount(opacity_only);
    start(&mut ui, flag);
    assert_eq!(
        seen(&mut ui).color,
        WHITE,
        "transition-opacity skips colors"
    );
}

#[test]
fn tailwind_transition_groups_animate_only_their_properties() {
    Owner::scope("motion-tailwind-groups", || {
        near(
            opacity_at(square().transition(Transition::colors()), 0.075),
            1.0,
            "transition-colors skips opacity",
        );
        near(
            opacity_at(square().transition(Transition::transform()), 0.075),
            1.0,
            "transition-transform skips opacity",
        );
        colors_transition_animates_the_background();
        transform_transition_animates_translate();
        opacity_transition_skips_colors();
    });
}

fn preset(keyframes: Keyframes) -> Harness {
    let flag = signal(false);
    let mut ui = mount(square().paint_when(flag, move |state| state.animate(keyframes)));
    start(&mut ui, flag);
    ui
}

#[test]
fn animate_spin_turns_once_a_second_forever() {
    Owner::scope("motion-spin", || {
        let mut ui = preset(Keyframes::spin());
        run(&mut ui, 0.25);
        near(
            seen(&mut ui).rotation(),
            FRAC_PI_2,
            "a quarter turn at 250ms",
        );
        run(&mut ui, 0.125);
        near(
            seen(&mut ui).rotation(),
            0.75 * PI,
            "linear: 135 degrees at 375ms",
        );
        run(&mut ui, 0.875);
        near(
            seen(&mut ui).rotation(),
            FRAC_PI_2,
            "a quarter turn again at 1250ms",
        );
    });
}

#[test]
fn animate_ping_scales_to_double_and_fades_by_75_percent() {
    Owner::scope("motion-ping", || {
        let mut ui = preset(Keyframes::ping());
        run(&mut ui, 0.375);
        let mid = seen(&mut ui);
        near(mid.scale(), 1.839_245, "scale halfway to the 75% stop");
        near(mid.opacity, 0.160_755, "opacity halfway to the 75% stop");
        run(&mut ui, 1.0);
        near(
            seen(&mut ui).scale(),
            1.839_245,
            "the next iteration repeats",
        );
    });
}

#[test]
fn animate_pulse_dips_to_half_opacity_every_two_seconds() {
    Owner::scope("motion-pulse", || {
        let mut ui = preset(Keyframes::pulse());
        run(&mut ui, 0.25);
        near(
            seen(&mut ui).opacity,
            1.0 - 0.5 * 0.135_307,
            "a quarter of the way down",
        );
        run(&mut ui, 0.75);
        near(seen(&mut ui).opacity, 0.5, "the 50% stop");
        run(&mut ui, 0.25);
        near(
            seen(&mut ui).opacity,
            0.5 + 0.5 * 0.135_307,
            "a quarter of the way back",
        );
        run(&mut ui, 1.0);
        near(
            seen(&mut ui).opacity,
            1.0 - 0.5 * 0.135_307,
            "the next iteration repeats",
        );
    });
}

#[test]
fn animate_bounce_drops_a_quarter_of_its_height() {
    Owner::scope("motion-bounce", || {
        let mut ui = preset(Keyframes::bounce());
        run(&mut ui, 0.125);
        near_px(
            seen(&mut ui).translate_y(),
            -9.642_16,
            "falling on cubic-bezier(0.8,0,1,1)",
        );
        run(&mut ui, 0.5);
        near_px(
            seen(&mut ui).translate_y(),
            -5.775_73,
            "rising on cubic-bezier(0,0,0.2,1)",
        );
        run(&mut ui, 1.0);
        near_px(
            seen(&mut ui).translate_y(),
            -5.775_73,
            "the next iteration repeats",
        );
    });
}
