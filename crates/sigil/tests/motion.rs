#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic, reason = "a test fails loudly on a missing value")]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel sizes and step counts are small"
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::time::Duration;

use sigil::animation::{Easing, Keyframes, Property};
use sigil::app::FrameHooks;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::{LayerEffect, SceneCommand};
use sigil_kittest::Harness;

const VIEW: (f32, f32) = (200.0, 200.0);
const SIDE: f32 = 40.0;
const STEPS_PER_SECOND: f32 = 16.0;
const GREY: Rgba8 = Rgba8 {
    r: 128,
    g: 128,
    b: 128,
    a: 255,
};
const BLACK: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
const WHITE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

#[derive(Clone, Copy, Debug)]
struct Seen {
    opacity: f32,
    color: Rgba8,
    matrix: [[f32; 3]; 2],
}

impl Seen {
    const fn translate_x(&self) -> f32 {
        self.matrix[0][2]
    }

    const fn translate_y(&self) -> f32 {
        self.matrix[1][2]
    }

    fn scale(&self) -> f32 {
        self.matrix[0][0].hypot(self.matrix[1][0])
    }

    fn rotation(&self) -> f32 {
        self.matrix[1][0].atan2(self.matrix[0][0])
    }
}

fn square() -> NodeBuilder {
    container()
        .bg(GREY)
        .layout(LayoutStyle::block().fixed_size(SIDE, SIDE))
}

fn mount(node: NodeBuilder) -> Harness {
    let mut ui = Harness::new(column().child(node)).pixels(200, 200);
    ui.settle();
    ui
}

fn run(ui: &mut Harness, seconds: f32) {
    let steps = (seconds * STEPS_PER_SECOND).round() as u32;
    for _ in 0..steps.max(1) {
        let _ = ui
            .host()
            .frame(VIEW, seconds / steps.max(1) as f32, FrameHooks::default());
    }
}

fn start(ui: &mut Harness, flag: Signal<bool>) {
    flag.set(true);
    let _ = ui.host().frame(VIEW, 0.0, FrameHooks::default());
}

const IDENTITY: [[f32; 3]; 2] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

fn seen(ui: &mut Harness) -> Seen {
    let scene = ui.host().emit(VIEW);
    let mut layers = vec![(1.0_f32, IDENTITY)];
    for command in &scene.commands {
        let (opacity, matrix) = *layers.last().unwrap();
        match command {
            SceneCommand::PushLayer(scope) => {
                let matrix = match scope.effect {
                    LayerEffect::Transform(transform) => transform,
                    _ => matrix,
                };
                layers.push((opacity * scope.opacity, matrix));
            }
            SceneCommand::SaveLayer { opacity: layer } => layers.push((opacity * layer, matrix)),
            SceneCommand::PushTransform(transform) => layers.push((opacity, *transform)),
            SceneCommand::PopLayer | SceneCommand::RestoreLayer | SceneCommand::PopTransform => {
                layers.pop();
            }
            SceneCommand::Rect(rect)
                if (rect.rect.width - SIDE).abs() < 0.5
                    && (rect.rect.height - SIDE).abs() < 0.5 =>
            {
                return Seen {
                    opacity: opacity * f32::from(rect.opacity) / 10_000.0,
                    color: rect.color,
                    matrix,
                };
            }
            _ => {}
        }
    }
    panic!("the square was not painted: {:?}", scene.commands);
}

#[track_caller]
fn near(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() < 2e-3,
        "{what}: got {actual}, want {expected}"
    );
}

#[track_caller]
fn near_px(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() < 0.05,
        "{what}: got {actual}, want {expected}"
    );
}

#[track_caller]
fn near_byte(actual: u8, expected: u8, what: &str) {
    assert!(
        actual.abs_diff(expected) <= 1,
        "{what}: got {actual}, want {expected}"
    );
}

fn animated(keyframes: Keyframes, flag: Signal<bool>) -> NodeBuilder {
    square()
        .opacity(0.4)
        .paint_when(flag, move |state| state.animate(keyframes))
}

#[test]
fn a_keyframe_animation_samples_its_stops_and_easing() {
    Owner::scope("motion-keyframes", || {
        let flag = signal(false);
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::IN_OUT)
            .keyframe(0.0, Property::Opacity(0.0))
            .keyframe(0.5, Property::Opacity(1.0))
            .keyframe(0.0, Property::TranslateX(0.0))
            .keyframe(1.0, Property::TranslateX(100.0))
            .keyframe(0.0, Property::Background(BLACK.into()))
            .keyframe(1.0, Property::Background(WHITE.into()));
        let mut ui = mount(animated(keyframes, flag));
        near(seen(&mut ui).opacity, 0.4, "before the animation starts");
        start(&mut ui, flag);

        run(&mut ui, 0.25);
        let quarter = seen(&mut ui);
        near(quarter.opacity, 0.775_561, "opacity at 25%");
        near_px(quarter.translate_x(), 23.658_7, "translate at 25%");
        near_byte(quarter.color.r, 60, "background at 25%");

        run(&mut ui, 0.25);
        let half = seen(&mut ui);
        near(half.opacity, 1.0, "opacity at its 50% stop");
        near_px(half.translate_x(), 77.556_1, "translate at 50%");

        run(&mut ui, 0.25);
        let late = seen(&mut ui);
        near(
            late.opacity,
            0.534_663,
            "an implicit 100% stop takes the base opacity",
        );
        near_px(late.translate_x(), 95.936_8, "translate at 75%");

        run(&mut ui, 0.5);
        let done = seen(&mut ui);
        near(
            done.opacity,
            0.4,
            "a finished animation shows the base style",
        );
        assert_eq!(
            done.matrix, IDENTITY,
            "a finished animation leaves no transform"
        );
        assert_eq!(
            done.color, GREY,
            "a finished animation shows the base color"
        );
    });
}

#[test]
fn a_keyframe_easing_applies_to_the_segment_that_starts_there() {
    Owner::scope("motion-keyframe-easing", || {
        let flag = signal(false);
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .keyframe(0.0, Property::Opacity(0.0))
            .keyframe(0.5, Property::Opacity(1.0))
            .keyframe(1.0, Property::Opacity(0.0))
            .ease_at(0.5, Easing::OUT);
        let mut ui = mount(animated(keyframes, flag));
        start(&mut ui, flag);

        run(&mut ui, 0.25);
        near(
            seen(&mut ui).opacity,
            0.802_403,
            "the first segment uses the CSS ease curve",
        );
        run(&mut ui, 0.5);
        near(
            seen(&mut ui).opacity,
            1.0 - 0.839_245,
            "the second segment uses its own easing",
        );
    });
}

#[test]
fn iterations_and_alternate_direction_replay_the_stops() {
    Owner::scope("motion-iterations", || {
        let flag = signal(false);
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(0.0, Property::TranslateX(0.0))
            .keyframe(1.0, Property::TranslateX(100.0))
            .iterations(3)
            .alternate();
        let mut ui = mount(animated(keyframes, flag));
        start(&mut ui, flag);

        run(&mut ui, 0.25);
        near_px(seen(&mut ui).translate_x(), 25.0, "first pass runs forward");
        run(&mut ui, 1.0);
        near_px(
            seen(&mut ui).translate_x(),
            75.0,
            "second pass runs backward",
        );
        run(&mut ui, 1.0);
        near_px(seen(&mut ui).translate_x(), 25.0, "third pass runs forward");
        run(&mut ui, 1.0);
        assert_eq!(seen(&mut ui).matrix, IDENTITY, "three passes then stop");
    });
}

#[test]
fn a_reversed_animation_starts_at_its_last_stop() {
    Owner::scope("motion-reverse", || {
        let flag = signal(false);
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(0.0, Property::TranslateX(0.0))
            .keyframe(1.0, Property::TranslateX(100.0))
            .reverse();
        let mut ui = mount(animated(keyframes, flag));
        start(&mut ui, flag);

        run(&mut ui, 0.25);
        near_px(
            seen(&mut ui).translate_x(),
            75.0,
            "reverse plays from 100% to 0%",
        );
    });
}

fn transitioned(flags: [Signal<bool>; 2], node: NodeBuilder) -> NodeBuilder {
    let [first, second] = flags;
    node.paint_when(first, |state| state.opacity(1.0))
        .paint_when(second, |state| state.opacity(0.4))
}

#[test]
fn a_transition_starts_from_the_current_value_when_the_style_changes_mid_flight() {
    Owner::scope("motion-retarget", || {
        let flags = [signal(false), signal(false)];
        let node = square()
            .opacity(0.2)
            .transition_opacity()
            .duration(Duration::from_secs(1))
            .ease(Easing::Linear);
        let mut ui = mount(transitioned(flags, node));
        start(&mut ui, flags[0]);
        run(&mut ui, 0.5);
        near(seen(&mut ui).opacity, 0.6, "halfway from 0.2 to 1.0");

        start(&mut ui, flags[1]);
        near(
            seen(&mut ui).opacity,
            0.6,
            "the new transition starts where the old one was",
        );
        run(&mut ui, 0.25);
        near(
            seen(&mut ui).opacity,
            0.55,
            "a quarter of the way from 0.6 to 0.4",
        );
        run(&mut ui, 1.0);
        near(seen(&mut ui).opacity, 0.4, "the new target");
    });
}

#[test]
fn a_transition_delay_holds_the_start_value() {
    Owner::scope("motion-delay", || {
        let flags = [signal(false), signal(false)];
        let node = square()
            .opacity(0.2)
            .transition_opacity()
            .duration(Duration::from_millis(500))
            .delay(Duration::from_millis(250))
            .ease(Easing::Linear);
        let mut ui = mount(transitioned(flags, node));
        start(&mut ui, flags[0]);
        run(&mut ui, 0.125);
        near(seen(&mut ui).opacity, 0.2, "inside the delay");
        run(&mut ui, 0.125);
        near(seen(&mut ui).opacity, 0.2, "at the end of the delay");
        run(&mut ui, 0.25);
        near(
            seen(&mut ui).opacity,
            0.6,
            "half the duration after the delay",
        );
    });
}

#[path = "motion/tailwind.rs"]
mod tailwind;
