#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "frame counts are small and positive"
)]

use core::f32::consts::TAU;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use sigil::animation::{Easing, Keyframes, Property, Transition};
use sigil::animation::{running_clocks, tick};
use sigil::app::FrameHooks;
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::{LayerEffect, SceneCommand};
use sigil_kittest::Harness;

const VIEW: (f32, f32) = (200.0, 200.0);
const SIDE: f32 = 40.0;
const FRAME: f32 = 1.0 / 60.0;
const IDENTITY: [[f32; 3]; 2] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

#[derive(Clone, Copy, Debug)]
struct Seen {
    opacity: f32,
    color: Color,
    matrix: [[f32; 3]; 2],
}

impl Seen {
    fn rotation(&self) -> f32 {
        self.matrix[1][0].atan2(self.matrix[0][0])
    }
}

fn square() -> NodeBuilder {
    container()
        .background(BLACK)
        .layout(LayoutStyle::block().fixed_size(SIDE, SIDE))
}

fn mount(node: NodeBuilder) -> Harness {
    let mut ui = Harness::new(column().child(node)).pixels(200, 200);
    ui.settle();
    ui
}

fn frames(ui: &mut Harness, count: u32, dt: f32) {
    for _ in 0..count {
        let _ = ui.host().frame(VIEW, dt, FrameHooks::default());
    }
}

fn run(ui: &mut Harness, seconds: f32) {
    let count = (seconds / FRAME).round() as u32;
    frames(ui, count.max(1), seconds / count.max(1) as f32);
}

fn start(ui: &mut Harness, flag: Signal<bool>) {
    flag.set(true);
    frames(ui, 1, 0.0);
}

fn layer(
    command: &SceneCommand,
    (opacity, matrix): (f32, [[f32; 3]; 2]),
) -> Option<(f32, [[f32; 3]; 2])> {
    match command {
        SceneCommand::PushLayer(scope) => {
            let matrix = match *scope.effect() {
                LayerEffect::Transform(transform) => transform,
                _ => matrix,
            };
            Some((opacity * scope.alpha(), matrix))
        }
        SceneCommand::SaveLayer { opacity: layer } => Some((opacity * layer, matrix)),
        SceneCommand::PushTransform(transform) => Some((opacity, *transform)),
        _ => None,
    }
}

fn seen(ui: &mut Harness) -> Seen {
    let scene = ui.host().emit(VIEW);
    let mut layers = vec![(1.0_f32, IDENTITY)];
    for command in scene.commands() {
        let top = *layers.last().unwrap();
        if let Some(pushed) = layer(command, top) {
            layers.push(pushed);
            continue;
        }
        match command {
            SceneCommand::PopLayer | SceneCommand::RestoreLayer | SceneCommand::PopTransform => {
                layers.pop();
            }
            SceneCommand::Rect(rect)
                if (rect.rect().width - SIDE).abs() < 0.5
                    && (rect.rect().height - SIDE).abs() < 0.5 =>
            {
                return Seen {
                    opacity: top.0 * rect.alpha(),
                    color: rect.color(),
                    matrix: top.1,
                };
            }
            _ => {}
        }
    }
    panic!("the square was not painted: {:?}", scene.commands());
}

fn animated(keyframes: Keyframes, flag: Signal<bool>) -> NodeBuilder {
    square().paint_when(flag, move |state| state.animation(keyframes))
}

fn turned(from: f32, to: f32) -> f32 {
    ((to - from) / TAU).rem_euclid(1.0)
}

#[test]
fn a_view_with_a_spinner_settles() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("review-spin-settles", || {
        let mut ui =
            Harness::new(column().child(square().animation(Keyframes::spin()))).pixels(200, 200);
        ui.settle();
        assert!(running_clocks() > 0, "the spinner still runs after settle");
        let before = seen(&mut ui).rotation();
        run(&mut ui, 0.25);
        let quarter = turned(before, seen(&mut ui).rotation());
        assert!(
            (quarter - 0.25).abs() < 0.01,
            "the spinner turns after settle: {quarter}"
        );
    });
}

#[test]
fn keyframe_and_transition_colors_meet_at_the_same_midpoint() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("review-color-space", || {
        let flag = signal(false);
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(0.0, Property::Background(BLACK.into()))
            .keyframe(1.0, Property::Background(WHITE.into()));
        let mut ui = mount(animated(keyframes, flag));
        start(&mut ui, flag);
        run(&mut ui, 0.5);
        let animated = seen(&mut ui).color.r;

        let toggle = signal(false);
        let node = square()
            .transition(
                Transition::colors()
                    .duration(Duration::from_secs(1))
                    .ease(Easing::Linear),
            )
            .paint_when(toggle, |state| state.background(WHITE));
        let mut fading = mount(node);
        start(&mut fading, toggle);
        run(&mut fading, 0.5);
        let transitioned = seen(&mut fading).color.r;

        assert!(
            animated.abs_diff(transitioned) <= 2,
            "black to white at 50%: animate gives {animated}, transition_colors gives {transitioned}"
        );
        assert!(
            animated.abs_diff(128) <= 2,
            "CSS mixes legacy colors in sRGB: got {animated}"
        );
    });
}

#[test]
fn an_infinite_spin_keeps_turning_after_two_to_the_nineteen_seconds() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("review-long-spin", || {
        let flag = signal(false);
        let mut ui = mount(animated(Keyframes::spin(), flag));
        start(&mut ui, flag);
        tick(524_288.0 - 0.25);
        frames(&mut ui, 1, 0.0);
        let mut last = seen(&mut ui).rotation();
        for window in 0..3 {
            frames(&mut ui, 15, FRAME);
            let now = seen(&mut ui).rotation();
            let turn = turned(last, now);
            assert!(
                (turn - 0.25).abs() < 0.01,
                "window {window}: turned {turn}, want 0.25"
            );
            last = now;
        }
    });
}

#[test]
fn a_zero_duration_infinite_animation_stops_its_clock() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("review-zero-duration", || {
        let flag = signal(false);
        let keyframes = Keyframes::new(Duration::ZERO)
            .keyframe(1.0, Property::Opacity(0.0))
            .infinite();
        let mut ui = mount(animated(keyframes, flag));
        start(&mut ui, flag);
        frames(&mut ui, 3, FRAME);
        assert_eq!(
            running_clocks(),
            0,
            "a zero-length animation keeps no clock running"
        );
        assert_eq!(seen(&mut ui).opacity, 1.0, "it shows the node's own style");
    });
}

#[test]
fn a_four_hundred_percent_shift_moves_four_widths() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("review-percent-range", || {
        let flag = signal(false);
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(0.0, Property::TranslateXPercent(400.0))
            .keyframe(1.0, Property::TranslateXPercent(400.0));
        let mut ui = mount(animated(keyframes, flag));
        start(&mut ui, flag);
        run(&mut ui, 0.5);
        let moved = seen(&mut ui).matrix[0][2];
        assert!(
            4.0f32.mul_add(-SIDE, moved).abs() < 0.5,
            "400% of 40 px: got {moved}, want 160"
        );
    });
}

#[test]
fn a_cubic_bezier_with_nan_points_eases_linearly() {
    let _keyframes = crate::serial::keyframes();
    let built = Easing::cubic_bezier(0.5, f32::NAN, 0.5, 1.0).apply(0.5);
    assert_eq!(built, 0.5, "NaN control points fall back to linear");
    let raw = Easing::CubicBezier {
        x1: 0.5,
        y1: f32::NAN,
        x2: f32::INFINITY,
        y2: 1.0,
    };
    assert_eq!(
        raw.apply(0.5),
        0.5,
        "a hand-built curve with NaN points is linear"
    );
    let at_nan = Easing::IN_OUT.apply(f32::NAN);
    assert_eq!(at_nan, 0.0, "a NaN progress is the start of the curve");
}

#[test]
fn a_zero_duration_transition_waits_for_its_delay() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("review-zero-delay", || {
        let flag = signal(false);
        let node = square()
            .opacity(0.2)
            .transition(
                Transition::opacity()
                    .duration(Duration::ZERO)
                    .delay(Duration::from_millis(250)),
            )
            .paint_when(flag, |state| state.opacity(1.0));
        let mut ui = mount(node);
        start(&mut ui, flag);
        run(&mut ui, 0.125);
        let held = seen(&mut ui).opacity;
        assert!(
            (held - 0.2).abs() < 2e-3,
            "125ms into a 250ms delay: got {held}, want 0.2"
        );
        run(&mut ui, 0.25);
        let after = seen(&mut ui).opacity;
        assert!(
            (after - 1.0).abs() < 2e-3,
            "after the delay: got {after}, want 1"
        );
    });
}

fn slide(index: u16) -> Keyframes {
    Keyframes::new(Duration::from_secs(1))
        .ease(Easing::Linear)
        .keyframe(0.0, Property::TranslateX(f32::from(index) / 1_024.0))
        .keyframe(1.0, Property::TranslateX(100.0))
}

#[test]
fn the_first_of_three_hundred_held_animations_still_plays() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("review-held-animations", || {
        let flags: Vec<Signal<bool>> = (0..300).map(|_| signal(false)).collect();
        let mut list = column();
        for (index, flag) in (0_u16..).zip(&flags) {
            let keyframes = slide(index);
            list = list.child(square().paint_when(*flag, move |state| state.animation(keyframes)));
        }
        let mut ui = Harness::new(list).pixels(200, 200);
        ui.settle();
        start(&mut ui, flags[0]);
        run(&mut ui, 0.5);
        let moved = seen(&mut ui).matrix[0][2];
        assert!(
            (moved - 50.0).abs() < 0.5,
            "the first held animation: moved {moved}, want 50"
        );
    });
}

struct Warnings(Arc<AtomicUsize>);

impl tracing::Subscriber for Warnings {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let meta = event.metadata();
        if meta.target() == "sigil::animation" && *meta.level() == tracing::Level::WARN {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn enter(&self, _: &tracing::span::Id) {}

    fn exit(&self, _: &tracing::span::Id) {}
}

#[test]
fn an_unscoped_animation_that_was_evicted_warns() {
    let _keyframes = crate::serial::keyframes();
    let warnings = Arc::new(AtomicUsize::new(0));
    tracing::subscriber::with_default(Warnings(Arc::clone(&warnings)), || {
        let owner = Owner::new("review-unscoped", None);
        let flag = owner.with(|| signal(false));
        let node = square().paint_when(flag, |state| state.animation(slide(1_000)));
        for index in 1_001..1_400 {
            drop(container().animation(slide(index)));
        }
        owner.with(|| {
            let mut ui = Harness::new(column().child(node)).pixels(200, 200);
            ui.settle();
            start(&mut ui, flag);
            run(&mut ui, 0.25);
        });
        owner.dispose();
    });
    assert_eq!(
        warnings.load(Ordering::Relaxed),
        1,
        "an evicted animation warns once"
    );
}
