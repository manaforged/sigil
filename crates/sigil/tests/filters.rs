#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, reason = "the checks print their measures")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates are small"
)]

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::kernel::container;
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{
    BlendMode, CommandScene, CornerRadii, DrawRect, Filter, LayerEffect, LayerScope, SceneBuilder,
    Shadow,
};
use sigil::style::Color;
use sigil::widgets::canvas;
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, MAX_MEAN_DIFF, MIN_CLOSE_FRACTION, cpu_rgba, diff};

#[path = "filters/model.rs"]
mod model;
#[path = "filters/perf.rs"]
mod perf;
#[path = "filters/style.rs"]
mod style;
#[path = "filters/text.rs"]
mod text;

use model::{expected_bytes, gaussian_cdf, linear};

pub(crate) const SIDE: u32 = 60;
pub(crate) const SCALE: f32 = 2.0;
const INPUT: Rgba8 = rgba(200, 80, 40, 255);
const BLACK: Rgba8 = rgba(0, 0, 0, 255);
const WHITE: Rgba8 = rgba(255, 255, 255, 255);
const RED: Rgba8 = rgba(255, 0, 0, 255);

pub(crate) const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

pub(crate) type Content = fn(&mut SceneBuilder, Rect);

#[derive(Clone)]
pub(crate) struct Card {
    pub(crate) filters: Vec<Filter>,
    pub(crate) content: Content,
}

pub(crate) fn filtered(scene: &mut SceneBuilder, rect: Rect, filters: &[Filter], content: Content) {
    let scope = LayerScope {
        clip: rect,
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Filter(filters.into()),
        opacity: 1.0,
        blend: BlendMode::default(),
    };
    scene.with_composite_layer(scope, |scene| content(scene, rect));
}

fn node(card: Card) -> impl IntoNode {
    let paint = canvas(move |scene: &mut SceneBuilder, rect: Rect| {
        scene.draw_rect(DrawRect::new(rect, BLACK));
        if card.filters.is_empty() {
            (card.content)(scene, rect);
        } else {
            filtered(scene, rect, &card.filters, card.content);
        }
    })
    .width(SIDE as f32)
    .height(SIDE as f32);
    container()
        .layout(LayoutStyle::block().fixed_size(SIDE as f32, SIDE as f32))
        .child(paint)
}

fn solid(scene: &mut SceneBuilder, rect: Rect) {
    scene.draw_rect(DrawRect::new(rect, INPUT));
}

#[track_caller]
pub(crate) fn gpu_frame(ui: &mut Harness) -> image::RgbaImage {
    match ui.image() {
        Ok(Some(frame)) => frame,
        Ok(None) => panic!("no GPU adapter: the harness returned no frame"),
        Err(err) => panic!("harness image failed: {err}"),
    }
}

pub(crate) fn cpu_frame(scene: &CommandScene, width: u32) -> image::RgbaImage {
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

pub(crate) fn frames_of<N: IntoNode + 'static>(
    app: impl Fn() -> N + Clone + 'static,
) -> Vec<(&'static str, image::RgbaImage)> {
    let mut pixels = Harness::new(app.clone())
        .unaudited()
        .pixels(SIDE, SIDE)
        .scale(SCALE);
    let mut surface = Harness::new(app).unaudited().gpu(SIDE, SIDE, SCALE);
    let scene = surface.host().emit((SIDE as f32, SIDE as f32));
    vec![
        ("pixels", gpu_frame(&mut pixels)),
        ("gpu", gpu_frame(&mut surface)),
        ("cpu", cpu_frame(&scene, SIDE)),
    ]
}

fn frames(card: &Card) -> Vec<(&'static str, image::RgbaImage)> {
    let card = card.clone();
    frames_of(move || node(card.clone()))
}

pub(crate) fn probe(frame: &image::RgbaImage, (x, y): (f32, f32)) -> image::Rgba<u8> {
    *frame.get_pixel((x * SCALE) as u32, (y * SCALE) as u32)
}

#[track_caller]
fn assert_color(filters: &[Filter], expected: [u8; 3]) {
    let card = Card {
        filters: filters.to_vec(),
        content: solid,
    };
    for (name, frame) in frames(&card) {
        let found = probe(&frame, (30.0, 30.0));
        println!("{name}: {filters:?} probe {found:?} model {expected:?}");
        let close = found
            .0
            .iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= 2);
        assert!(
            close,
            "{name}: {filters:?} gave {found:?}, the Filter Effects model gives {expected:?} (±2/255)"
        );
    }
}

#[test]
fn each_color_filter_yields_the_spec_matrix_result() {
    let cases = [
        Filter::Brightness(0.5),
        Filter::Brightness(1.6),
        Filter::Contrast(0.5),
        Filter::Contrast(2.0),
        Filter::Grayscale(1.0),
        Filter::Grayscale(0.5),
        Filter::HueRotate(90.0),
        Filter::HueRotate(-200.0),
        Filter::Invert(1.0),
        Filter::Invert(0.25),
        Filter::Saturate(0.3),
        Filter::Saturate(2.5),
        Filter::Sepia(1.0),
        Filter::Sepia(0.6),
        Filter::Opacity(0.5),
    ];
    for filter in cases {
        assert_color(&[filter], expected_bytes(&[filter], INPUT));
    }
}

#[test]
fn filter_order_matters_as_the_spec_applies_the_list_in_order() {
    let forward = [Filter::Grayscale(1.0), Filter::Sepia(1.0)];
    let reverse = [Filter::Sepia(1.0), Filter::Grayscale(1.0)];
    let (a, b) = (
        expected_bytes(&forward, INPUT),
        expected_bytes(&reverse, INPUT),
    );
    assert!(
        a.iter().zip(b).any(|(x, y)| x.abs_diff(y) > 20),
        "the model orders differ: {a:?} vs {b:?}"
    );
    assert_color(&forward, a);
    assert_color(&reverse, b);
    let clamped = [Filter::Brightness(3.0), Filter::Brightness(0.5)];
    assert_color(&clamped, expected_bytes(&clamped, INPUT));
}

fn left_half_white(scene: &mut SceneBuilder, rect: Rect) {
    let half = Rect::new(rect.origin, rect.width * 0.5, rect.height);
    scene.draw_rect(DrawRect::new(half, WHITE));
}

#[test]
fn blur_spreads_coverage_as_a_gaussian_with_the_css_standard_deviation() {
    let sigma = 4.0;
    let card = Card {
        filters: vec![Filter::Blur(sigma)],
        content: left_half_white,
    };
    for (name, frame) in frames(&card) {
        for x in [22.0, 26.0, 30.0, 34.0, 38.0] {
            let center = x + 0.5 / SCALE;
            let expected = gaussian_cdf((30.0 - center) / sigma);
            let found = linear(probe(&frame, (x, 30.0)).0[0]);
            println!("{name}: x {x} coverage {found:.3} gaussian {expected:.3}");
            assert!(
                (found - expected).abs() <= 0.04,
                "{name}: at x {x} the blurred edge covers {found:.3}, a Gaussian with sigma {sigma} gives {expected:.3}"
            );
        }
    }
}

fn red_square(scene: &mut SceneBuilder, rect: Rect) {
    let square = Rect::new(
        Point::new(rect.origin.x + 10.0, rect.origin.y + 10.0),
        20.0,
        20.0,
    );
    scene.draw_rect(DrawRect::new(square, RED));
}

fn shadow_card(alpha: f32) -> Card {
    let shadow = Shadow::new([12.0, 12.0], 1.0, 0.0, [0.0, 0.0, 1.0, alpha]);
    Card {
        filters: vec![Filter::DropShadow(shadow)],
        content: red_square,
    }
}

#[test]
fn drop_shadow_shows_the_tint_at_the_offset_under_the_source() {
    for alpha in [1.0, 0.5] {
        for (name, frame) in frames(&shadow_card(alpha)) {
            let shade = probe(&frame, (36.0, 36.0)).0;
            let [r, g, b] = [shade[0], shade[1], shade[2]].map(linear);
            println!("{name}: alpha {alpha} shadow probe linear {r:.3} {g:.3} {b:.3}");
            assert!(
                (b - alpha).abs() <= 0.05 && r <= 0.02 && g <= 0.02,
                "{name}: the shadow at the offset shows blue {b:.3}, the tint alpha is {alpha}"
            );
            for point in [(25.0, 25.0), (15.0, 15.0)] {
                let top = probe(&frame, point).0;
                assert!(
                    top[0] >= 250 && top[2] <= 3,
                    "{name}: the source stays on top of its shadow at {point:?}: {top:?}"
                );
            }
            let clear = probe(&frame, (50.0, 50.0)).0;
            assert!(
                clear[..3].iter().all(|c| *c <= 2),
                "{name}: no shadow beyond its reach: {clear:?}"
            );
        }
    }
}

fn gallery() -> impl IntoNode {
    let cards = [
        vec![Filter::Grayscale(1.0), Filter::Sepia(1.0)],
        vec![Filter::HueRotate(90.0), Filter::Saturate(2.0)],
        vec![Filter::Blur(3.0)],
        vec![Filter::Contrast(1.5), Filter::Invert(0.3)],
    ];
    let mut row =
        container().layout(LayoutStyle::flex_row().fixed_size(SIDE as f32 * 5.0, SIDE as f32));
    for filters in cards {
        row = row.child(node(Card {
            filters,
            content: solid,
        }));
    }
    row.child(node(shadow_card(0.8)))
}

#[test]
fn filtered_scenes_agree_on_the_cpu_and_the_gpu() {
    let (width, height) = (SIDE * 5, SIDE);
    let mut ui = Harness::new(gallery).gpu(width, height, SCALE);
    let gpu = gpu_frame(&mut ui);
    let scene = ui.host().emit((width as f32, height as f32));
    let cpu = cpu_frame(&scene, width);
    let measured = diff(gpu.as_raw(), cpu.as_raw());
    println!(
        "filters cpu/gpu: mean {:.2}/255, {:.1}% close",
        measured.mean,
        measured.close * 100.0
    );
    assert!(
        measured.mean <= MAX_MEAN_DIFF && measured.close >= MIN_CLOSE_FRACTION,
        "cpu and gpu differ: {measured:?}, limits mean {MAX_MEAN_DIFF} close {MIN_CLOSE_FRACTION}"
    );
}
