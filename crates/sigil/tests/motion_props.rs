#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]
#![allow(clippy::print_stdout, reason = "the checks print their measures")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "frame counts and color channels are small and positive"
)]

use std::f32::consts::FRAC_PI_2;
use std::time::Duration;

use sigil::animation::{Easing, Keyframes, Property};
use sigil::app::FrameHooks;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::{Filter, LayerEffect, SceneCommand, Shadow};
use sigil::style::{Background, GradientAxis};
use sigil_kittest::Harness;

#[path = "motion_props/alloc.rs"]
mod alloc;
#[path = "motion_props/transitions.rs"]
mod transitions;

pub(crate) const VIEW: (f32, f32) = (200.0, 200.0);
pub(crate) const SIDE: f32 = 40.0;
const STEPS_PER_SECOND: f32 = 16.0;
pub(crate) const DISTANCE: f32 = 400.0;
pub(crate) const GREY: Color = rgba(128, 128, 128, 255);
pub(crate) const BLACK: Color = rgba(0, 0, 0, 255);
pub(crate) const WHITE: Color = rgba(255, 255, 255, 255);
pub(crate) const BLUE: Color = rgba(0, 0, 255, 255);

pub(crate) const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
}

pub(crate) fn shadow(y: f32, blur: f32, color: Color) -> Shadow {
    let linear = |byte: u8| {
        let unit = f32::from(byte) / 255.0;
        if unit <= 0.040_45 {
            unit / 12.92
        } else {
            ((unit + 0.055) / 1.055).powf(2.4)
        }
    };
    let alpha = f32::from(color.a) / 255.0;
    Shadow::new(
        [0.0, y],
        blur,
        0.0,
        [linear(color.r), linear(color.g), linear(color.b), alpha],
    )
}

#[allow(
    clippy::suboptimal_flops,
    reason = "the model spells out the sRGB spec formula"
)]
pub(crate) fn srgb_byte(linear: f32) -> u8 {
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(crate) fn square() -> NodeBuilder {
    container()
        .background(GREY)
        .layout(LayoutStyle::block().fixed_size(SIDE, SIDE))
}

pub(crate) fn gradient_square() -> NodeBuilder {
    container()
        .background(
            Background::linear(GradientAxis::LeftToRight)
                .start(BLACK)
                .end(BLUE),
        )
        .layout(LayoutStyle::block().fixed_size(SIDE, SIDE))
}

pub(crate) fn under_perspective(child: NodeBuilder) -> NodeBuilder {
    container()
        .perspective(DISTANCE)
        .layout(LayoutStyle::block().fixed_size(SIDE, SIDE))
        .child(child)
}

pub(crate) fn mount(node: NodeBuilder) -> Harness {
    let mut ui = Harness::new(column().child(node)).pixels(200, 200);
    ui.settle();
    ui
}

pub(crate) fn run(ui: &mut Harness, seconds: f32) {
    let steps = (seconds * STEPS_PER_SECOND).round() as u32;
    for _ in 0..steps.max(1) {
        let _ = ui
            .host()
            .frame(VIEW, seconds / steps.max(1) as f32, FrameHooks::default());
    }
}

pub(crate) fn start(ui: &mut Harness, flag: Signal<bool>) {
    flag.set(true);
    let _ = ui.host().frame(VIEW, 0.0, FrameHooks::default());
}

pub(crate) fn filters(ui: &mut Harness) -> Vec<Filter> {
    let scene = ui.host().emit(VIEW);
    scene
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::PushLayer(scope) => match &scope.effect {
                LayerEffect::Filter(list) => Some(list.to_vec()),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn stops(ui: &mut Harness) -> Vec<(f32, Color)> {
    let scene = ui.host().emit(VIEW);
    scene
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::Gradient(fill) => Some(
                fill.gradient
                    .stops
                    .iter()
                    .map(|stop| (stop.offset, stop.color))
                    .collect(),
            ),
            _ => None,
        })
        .expect("the gradient was not painted")
}

fn matrix(effect: &LayerEffect) -> Option<[[f32; 4]; 4]> {
    let LayerEffect::Projected { transform, .. } = effect else {
        return None;
    };
    let debug = format!("{transform:?}");
    let (_, body) = debug.split_once('(')?;
    let numbers: Vec<f32> = body
        .split(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | 'e')))
        .filter_map(|token| token.parse().ok())
        .collect();
    let mut rows = [[0.0; 4]; 4];
    for (index, value) in numbers.into_iter().take(16).enumerate() {
        rows[index / 4][index % 4] = value;
    }
    Some(rows)
}

fn project(rows: &[[f32; 4]; 4], x: f32, y: f32) -> f32 {
    let along = |row: [f32; 4]| row[0].mul_add(x, row[1].mul_add(y, row[3]));
    along(rows[0]) / along(rows[3])
}

pub(crate) fn width_ratio(ui: &mut Harness) -> f32 {
    let scene = ui.host().emit(VIEW);
    let mut rows = None;
    for command in &scene.commands {
        match command {
            SceneCommand::PushLayer(scope) if matrix(&scope.effect).is_some() => {
                rows = matrix(&scope.effect);
            }
            SceneCommand::Rect(rect) if (rect.rect.width - SIDE).abs() < 0.5 => {
                let Some(rows) = rows else {
                    return 1.0;
                };
                let (left, top) = (rect.rect.origin.x, rect.rect.origin.y);
                let middle = top + SIDE / 2.0;
                return (project(&rows, left + SIDE, middle) - project(&rows, left, middle)) / SIDE;
            }
            _ => {}
        }
    }
    panic!("the square was not painted: {:?}", scene.commands);
}

#[track_caller]
pub(crate) fn near(actual: f32, expected: f32, tolerance: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: got {actual}, want {expected}"
    );
}

#[track_caller]
pub(crate) fn near_color(actual: Color, expected: Color, what: &str) {
    let close = [
        (actual.r, expected.r),
        (actual.g, expected.g),
        (actual.b, expected.b),
        (actual.a, expected.a),
    ]
    .iter()
    .all(|(a, b)| a.abs_diff(*b) <= 1);
    assert!(close, "{what}: got {actual:?}, want {expected:?}");
}

pub(crate) fn amount(list: &[Filter], pick: fn(&Filter) -> Option<f32>) -> Option<f32> {
    list.iter().find_map(pick)
}

pub(crate) fn drop_shadow(list: &[Filter]) -> Shadow {
    list.iter()
        .find_map(|filter| match filter {
            Filter::DropShadow(shadow) => Some(*shadow),
            _ => None,
        })
        .expect("a drop shadow")
}

pub(crate) fn shadow_color(shadow: Shadow) -> Color {
    let [r, g, b, a] = shadow.color();
    rgba(
        srgb_byte(r),
        srgb_byte(g),
        srgb_byte(b),
        (a * 255.0).round() as u8,
    )
}

pub(crate) const fn blur(filter: &Filter) -> Option<f32> {
    if let Filter::Blur(px) = filter {
        Some(*px)
    } else {
        None
    }
}

pub(crate) const fn brightness(filter: &Filter) -> Option<f32> {
    if let Filter::Brightness(amount) = filter {
        Some(*amount)
    } else {
        None
    }
}

pub(crate) const fn hue(filter: &Filter) -> Option<f32> {
    if let Filter::HueRotate(degrees) = filter {
        Some(*degrees)
    } else {
        None
    }
}

fn played(node: NodeBuilder, keyframes: Keyframes) -> (Harness, Signal<bool>) {
    let flag = signal(false);
    let mut ui = mount(node.paint_when(flag, move |state| state.animation(keyframes)));
    start(&mut ui, flag);
    (ui, flag)
}

#[test]
fn filter_keyframes_sample_their_stops() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("props-filter-keyframes", || {
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(0.0, Property::Blur(0.0))
            .keyframe(1.0, Property::Blur(8.0))
            .keyframe(0.0, Property::HueRotate(0.0))
            .keyframe(1.0, Property::HueRotate(180.0))
            .keyframe(0.0, Property::DropShadow(shadow(0.0, 0.0, BLACK)))
            .keyframe(1.0, Property::DropShadow(shadow(8.0, 4.0, WHITE)));
        let (mut ui, _flag) = played(square(), keyframes);
        run(&mut ui, 0.25);
        let list = filters(&mut ui);
        near(amount(&list, blur).expect("blur"), 2.0, 0.04, "blur at 25%");
        near(
            amount(&list, hue).expect("hue"),
            45.0,
            0.5,
            "hue-rotate at 25%",
        );
        let drop = drop_shadow(&list);
        near(drop.offset()[1], 2.0, 0.04, "drop-shadow y at 25%");
        near(drop.blur(), 1.0, 0.04, "drop-shadow blur at 25%");
        near_color(
            shadow_color(drop),
            rgba(64, 64, 64, 255),
            "drop-shadow color at 25%",
        );
        run(&mut ui, 1.0);
        assert!(
            filters(&mut ui).is_empty(),
            "a finished animation leaves the node unfiltered"
        );
    });
}

#[test]
fn a_brightness_keyframe_follows_its_easing() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("props-brightness-easing", || {
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::IN_OUT)
            .keyframe(0.0, Property::Brightness(1.0))
            .keyframe(1.0, Property::Brightness(2.0));
        let (mut ui, _flag) = played(square(), keyframes);
        run(&mut ui, 0.25);
        let early = amount(&filters(&mut ui), brightness).expect("brightness");
        near(early, 1.236_587, 0.006, "brightness at 25% on ease-in-out");
        run(&mut ui, 0.25);
        let half = amount(&filters(&mut ui), brightness).expect("brightness");
        near(half, 1.775_561, 0.006, "brightness at 50% on ease-in-out");
    });
}

#[test]
fn a_gradient_stop_color_keyframe_mixes_in_srgb() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("props-stop-keyframes", || {
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(0.0, Property::GradientFrom(BLACK.into()))
            .keyframe(1.0, Property::GradientFrom(WHITE.into()))
            .keyframe(1.0, Property::GradientToPosition(50.0));
        let (mut ui, _flag) = played(gradient_square(), keyframes);
        run(&mut ui, 0.25);
        let found = stops(&mut ui);
        near_color(found[0].1, rgba(64, 64, 64, 255), "the from stop at 25%");
        near_color(found[1].1, BLUE, "the to stop keeps its color");
        near(found[1].0, 0.88, 0.006, "the to stop position at 25%");
    });
}

#[test]
fn a_rotate_y_keyframe_turns_the_node() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("props-rotate-y-keyframes", || {
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(0.0, Property::RotateY(0.0))
            .keyframe(1.0, Property::RotateY(FRAC_PI_2));
        let (mut ui, _flag) = played(square(), keyframes);
        run(&mut ui, 0.5);
        near(
            width_ratio(&mut ui),
            (FRAC_PI_2 / 2.0).cos(),
            2e-3,
            "rotateY(45deg) narrows by cos",
        );
    });
}

#[test]
fn a_translate_z_keyframe_moves_the_node_toward_the_viewer() {
    let _keyframes = crate::serial::keyframes();
    Owner::scope("props-translate-z-keyframes", || {
        let flag = signal(false);
        let keyframes = Keyframes::new(Duration::from_secs(1))
            .ease(Easing::Linear)
            .keyframe(0.0, Property::TranslateZ(0.0))
            .keyframe(1.0, Property::TranslateZ(200.0));
        let child = square().paint_when(flag, move |state| state.animation(keyframes));
        let mut ui = mount(under_perspective(child));
        start(&mut ui, flag);
        run(&mut ui, 0.5);
        let want = DISTANCE / (DISTANCE - 100.0);
        near(
            width_ratio(&mut ui),
            want,
            2e-3,
            "translateZ(100px) under perspective(400px)",
        );
    });
}
