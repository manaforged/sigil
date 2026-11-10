#![allow(clippy::print_stdout, reason = "the checks print their measures")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small"
)]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::kernel::{ImageSource, ObjectFit, container};
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{
    CommandScene, CornerRadii, DrawGradient, DrawImage, DrawRect, GradientStop, LayerEffect,
    LayerScope, LinearGradient, Mask, MaskMode, SceneBuilder,
};
use sigil::style::Color;
use sigil::widgets::canvas;
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, MAX_MEAN_DIFF, MIN_CLOSE_FRACTION, cpu_rgba, diff};

#[path = "masks/perf.rs"]
mod perf;
#[path = "masks/style.rs"]
mod style;

const SIDE: u32 = 60;
const SCALE: f32 = 2.0;
const GAP: f32 = 10.0;
const RED: Rgba8 = rgba(255, 0, 0, 255);
const BLUE: Rgba8 = rgba(0, 0, 255, 255);

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn sized(node: impl IntoNode + 'static, width: f32) -> impl IntoNode {
    container()
        .layout(LayoutStyle::block().fixed_size(width, SIDE as f32))
        .child(node)
}

fn ramp(rect: Rect, from: Rgba8, to: Rgba8) -> DrawGradient {
    let y = rect.height.mul_add(0.5, rect.origin.y);
    let start = Point::new(rect.origin.x, y);
    let end = Point::new(rect.origin.x + rect.width, y);
    let stops = [GradientStop::new(0.0, from), GradientStop::new(1.0, to)];
    DrawGradient::new(rect, LinearGradient::new(start, end, stops.to_vec()))
}

fn gradient_mask(mode: MaskMode, from: Rgba8, to: Rgba8, rect: Rect) -> Mask {
    let mut scene = SceneBuilder::new();
    scene.draw_gradient(ramp(rect, from, to));
    Mask::new(mode, scene.finish())
}

fn alpha_ramp(rect: Rect) -> Mask {
    gradient_mask(MaskMode::Alpha, rgba(0, 0, 0, 255), rgba(0, 0, 0, 0), rect)
}

fn luminance_ramp(rect: Rect) -> Mask {
    let white = rgba(255, 255, 255, 255);
    gradient_mask(MaskMode::Luminance, white, rgba(0, 0, 0, 255), rect)
}

fn half_opaque_image() -> ImageSource {
    let side = SIDE * 2;
    let mut pixels = Vec::with_capacity((side * side * 4) as usize);
    for _ in 0..side {
        for x in 0..side {
            let alpha = if x < side / 2 { 255 } else { 0 };
            pixels.extend_from_slice(&[40, 200, 90, alpha]);
        }
    }
    ImageSource::from_rgba8(side, side, pixels).expect("mask image")
}

fn image_mask(rect: Rect) -> Mask {
    let mut scene = SceneBuilder::new();
    scene.draw_image(DrawImage {
        node: None,
        source: half_opaque_image(),
        fit: ObjectFit::Fill,
        destination: rect,
        opacity: 10_000,
        crop: None,
        raster: None,
    });
    Mask::new(MaskMode::Alpha, scene.finish())
}

fn masked_red(scene: &mut SceneBuilder, rect: Rect, mask: Mask) {
    let scope = LayerScope {
        clip: rect,
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Mask(mask),
        opacity: 1.0,
        blend: sigil::scene::BlendMode::Normal,
    };
    scene.with_composite_layer(scope, |scene| scene.draw_rect(DrawRect::new(rect, RED)));
}

fn masked(mask: fn(Rect) -> Mask) -> impl IntoNode {
    let paint = canvas(move |scene: &mut SceneBuilder, rect: Rect| {
        let below = Rect::new(
            Point::new(rect.origin.x, rect.origin.y + GAP),
            rect.width,
            rect.height - GAP,
        );
        masked_red(scene, below, mask(below));
    })
    .width(SIDE as f32)
    .height(SIDE as f32);
    sized(paint, SIDE as f32)
}

fn alpha_masked() -> impl IntoNode {
    masked(alpha_ramp)
}

fn luminance_masked() -> impl IntoNode {
    masked(luminance_ramp)
}

fn image_masked() -> impl IntoNode {
    masked(image_mask)
}

fn overlap(scene: &mut SceneBuilder, rect: Rect) {
    let (x, y) = (rect.origin.x, rect.origin.y);
    scene.with_layer(0.5, |scene| {
        scene.draw_rect(DrawRect::new(
            Rect::new(Point::new(x + 5.0, y + 15.0), 30.0, 30.0),
            RED,
        ));
        scene.draw_rect(DrawRect::new(
            Rect::new(Point::new(x + 25.0, y + 15.0), 30.0, 30.0),
            BLUE,
        ));
    });
}

fn group() -> impl IntoNode {
    let paint = canvas(overlap).width(SIDE as f32).height(SIDE as f32);
    sized(paint, SIDE as f32)
}

#[track_caller]
fn gpu_frame(ui: &mut Harness) -> image::RgbaImage {
    match ui.image() {
        Ok(Some(frame)) => frame,
        Ok(None) => panic!("no GPU adapter: the harness returned no frame"),
        Err(err) => panic!("harness image failed: {err}"),
    }
}

fn cpu_frame(scene: &CommandScene, width: u32) -> image::RgbaImage {
    let (device_w, device_h) = ((width as f32 * SCALE) as u32, (SIDE as f32 * SCALE) as u32);
    let frame = Frame {
        scene,
        viewport: (width as f32, SIDE as f32),
        size: PhysicalSize::new(device_w, device_h),
        clear: Color::rgba(0.0, 0.0, 0.0, 1.0),
    };
    let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    image::RgbaImage::from_raw(device_w, device_h, rgba).expect("cpu frame")
}

fn frames<N: IntoNode + 'static>(app: fn() -> N) -> Vec<(&'static str, image::RgbaImage)> {
    let mut pixels = Harness::new(app).pixels(SIDE, SIDE).scale(SCALE);
    let mut surface = Harness::new(app).gpu(SIDE, SIDE, SCALE);
    let scene = surface.host().emit((SIDE as f32, SIDE as f32));
    vec![
        ("pixels", gpu_frame(&mut pixels)),
        ("gpu", gpu_frame(&mut surface)),
        ("cpu", cpu_frame(&scene, SIDE)),
    ]
}

fn probe(frame: &image::RgbaImage, (x, y): (f32, f32)) -> image::Rgba<u8> {
    *frame.get_pixel((x * SCALE) as u32, (y * SCALE) as u32)
}

fn linear(channel: u8) -> f32 {
    let value = f32::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn distance(a: image::Rgba<u8>, b: image::Rgba<u8>) -> f32 {
    a.0.iter()
        .zip(b.0)
        .take(3)
        .map(|(x, y)| (linear(*x) - linear(y)).abs())
        .sum()
}

fn coverage(frame: &image::RgbaImage, point: (f32, f32)) -> f32 {
    let background = probe(frame, (30.0, GAP * 0.5));
    let ink = image::Rgba([RED.r, RED.g, RED.b, RED.a]);
    distance(probe(frame, point), background) / distance(ink, background).max(f32::EPSILON)
}

#[track_caller]
fn assert_ramp<N: IntoNode + 'static>(app: fn() -> N, mid: Option<(f32, f32)>) {
    let measured: Vec<(&str, [f32; 3])> = frames(app)
        .iter()
        .map(|(name, frame)| {
            let at = |x: f32| coverage(frame, (x, 35.0));
            (*name, [at(3.0), at(30.0), at(57.0)])
        })
        .collect();
    println!("coverage left, middle, right: {measured:?}");
    for (name, [left, middle, right]) in measured {
        assert!(
            left > 0.85,
            "{name}: the left edge keeps the content (coverage {left:.2})"
        );
        assert!(
            right < 0.15,
            "{name}: the right edge is masked away (coverage {right:.2})"
        );
        if let Some((low, high)) = mid {
            assert!(
                (low..=high).contains(&middle),
                "{name}: the midpoint shows {middle:.2}, expected {low:.2}..={high:.2}"
            );
        }
    }
}

#[test]
fn an_alpha_gradient_mask_fades_the_content_from_left_to_right() {
    assert_ramp(alpha_masked, Some((0.4, 0.6)));
}

fn white_to_black() -> impl IntoNode {
    let paint = canvas(|scene: &mut SceneBuilder, rect: Rect| {
        scene.draw_gradient(ramp(rect, rgba(255, 255, 255, 255), rgba(0, 0, 0, 255)));
    })
    .width(SIDE as f32)
    .height(SIDE as f32);
    sized(paint, SIDE as f32)
}

fn luminance(pixel: image::Rgba<u8>) -> f32 {
    let [r, g, b, _] = pixel.0.map(|channel| f32::from(channel) / 255.0);
    0.0721f32.mul_add(b, 0.2125f32.mul_add(r, 0.7154 * g))
}

#[test]
fn a_luminance_gradient_mask_fades_the_content_through_luminance() {
    let drawn: Vec<_> = frames(white_to_black)
        .into_iter()
        .map(|(_, frame)| frame)
        .collect();
    for ((name, frame), gradient) in frames(luminance_masked).iter().zip(&drawn) {
        for x in [3.0, 30.0, 57.0] {
            let expected = luminance(probe(gradient, (x, 35.0)));
            let found = coverage(frame, (x, 35.0));
            println!("{name}: x {x} gradient luminance {expected:.3} coverage {found:.3}");
            assert!(
                (found - expected).abs() <= 0.06,
                "{name}: at x {x} the content shows {found:.3}, the mask's luminance is {expected:.3}"
            );
        }
        assert!(
            coverage(frame, (3.0, 35.0)) > 0.85,
            "{name}: the white end keeps the content"
        );
        assert!(
            coverage(frame, (59.5, 35.0)) < 0.2,
            "{name}: the black end masks it away"
        );
    }
}

fn gray_mask(rect: Rect) -> Mask {
    let mut scene = SceneBuilder::new();
    scene.draw_rect(DrawRect::new(rect, rgba(128, 128, 128, 255)));
    Mask::new(MaskMode::Luminance, scene.finish())
}

fn gray_masked() -> impl IntoNode {
    masked(gray_mask)
}

#[test]
fn a_mid_gray_luminance_mask_keeps_half_the_content() {
    for (name, frame) in frames(gray_masked) {
        let found = coverage(&frame, (30.0, 35.0));
        println!("{name}: gray luminance coverage {found:.3}");
        assert!(
            (0.44..=0.56).contains(&found),
            "{name}: sRGB gray 128 has luminance 0.50, the content shows {found:.3}"
        );
    }
}

#[test]
fn an_image_mask_follows_the_image_alpha() {
    assert_ramp(image_masked, None);
    for (name, frame) in frames(image_masked) {
        let (inside, outside) = (
            coverage(&frame, (27.0, 35.0)),
            coverage(&frame, (33.0, 35.0)),
        );
        assert!(
            inside > 0.9,
            "{name}: the opaque half keeps the content ({inside:.2})"
        );
        assert!(
            outside < 0.1,
            "{name}: the clear half removes the content ({outside:.2})"
        );
    }
}

#[test]
fn group_opacity_composites_overlapping_children_once() {
    for (name, frame) in frames(group) {
        let (red, both, blue) = (
            probe(&frame, (10.0, 30.0)),
            probe(&frame, (30.0, 30.0)),
            probe(&frame, (50.0, 30.0)),
        );
        println!("{name}: red {red:?} overlap {both:?} blue {blue:?}");
        let reddish = red.0[0].abs_diff(both.0[0]) > 40;
        assert!(
            reddish,
            "{name}: the red child {red:?} differs from the overlap {both:?}"
        );
        let same = both.0.iter().zip(blue.0).all(|(a, b)| a.abs_diff(b) <= 2);
        assert!(
            same,
            "{name}: the overlap {both:?} equals the blue-only area {blue:?}"
        );
    }
}

fn gallery() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_row().fixed_size(SIDE as f32 * 4.0, SIDE as f32))
        .child(alpha_masked())
        .child(luminance_masked())
        .child(image_masked())
        .child(group())
}

#[test]
fn masked_scenes_agree_on_the_cpu_and_the_gpu() {
    let (width, height) = (SIDE * 4, SIDE);
    let mut ui = Harness::new(gallery).gpu(width, height, SCALE);
    let gpu = gpu_frame(&mut ui);
    let scene = ui.host().emit((width as f32, height as f32));
    let cpu = cpu_frame(&scene, width);
    let measured = diff(gpu.as_raw(), cpu.as_raw());
    println!(
        "masks cpu/gpu: mean {:.2}/255, {:.1}% close",
        measured.mean,
        measured.close * 100.0
    );
    assert!(
        measured.mean <= MAX_MEAN_DIFF && measured.close >= MIN_CLOSE_FRACTION,
        "cpu and gpu differ: {measured:?}, limits mean {MAX_MEAN_DIFF} close {MIN_CLOSE_FRACTION}"
    );
}
