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
#![allow(clippy::panic, reason = "a test fails loudly on a missing value")]

use sigil::geometry::PhysicalSize;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::style::{Background, ClipShape, Color, GradientAxis, Radial, Transform2D};
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, cpu_rgba};

const SCALE: f32 = 2.0;
const RED: Rgba8 = rgb(255, 0, 0);
const GREEN: Rgba8 = rgb(0, 255, 0);
const BLUE: Rgba8 = rgb(0, 0, 255);
const WHITE: Rgba8 = rgb(255, 255, 255);
const BLACK: Rgba8 = rgb(0, 0, 0);

const fn rgb(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}

#[derive(Clone, Copy, Debug)]
enum Surface {
    Pixels,
    Gpu,
    Cpu,
}

const SURFACES: [Surface; 3] = [Surface::Pixels, Surface::Gpu, Surface::Cpu];

fn shoot<A: IntoNode + 'static>(
    app: impl Fn() -> A + 'static,
    w: u32,
    h: u32,
    surface: Surface,
) -> image::RgbaImage {
    let mut ui = match surface {
        Surface::Pixels => Harness::new(app).pixels(w, h).scale(SCALE),
        Surface::Gpu => Harness::new(app).gpu(w, h, SCALE),
        Surface::Cpu => {
            let mut ui = Harness::new(app).pixels(w, h).scale(SCALE);
            let viewport = (w as f32, h as f32);
            let scene = ui.host().emit(viewport);
            let size = PhysicalSize::new((w as f32 * SCALE) as u32, (h as f32 * SCALE) as u32);
            let frame = Frame {
                scene: &scene,
                viewport,
                size,
                clear: Color::rgba(1.0, 1.0, 1.0, 1.0),
            };
            let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
            return image::RgbaImage::from_raw(size.width, size.height, rgba)
                .expect("cpu frame size");
        }
    };
    match ui.image() {
        Ok(Some(frame)) => frame,
        Ok(None) => panic!("no frame"),
        Err(err) => panic!("harness image failed: {err}"),
    }
}

#[track_caller]
fn check(failures: &[String]) {
    for line in failures {
        println!("FAIL {line}");
    }
    assert!(
        failures.is_empty(),
        "{} probe(s) failed: {failures:#?}",
        failures.len()
    );
}

fn at(frame: &image::RgbaImage, x: f32, y: f32) -> [u8; 3] {
    let p = frame.get_pixel((x * SCALE) as u32, (y * SCALE) as u32);
    [p[0], p[1], p[2]]
}

fn off(found: [u8; 3], want: Rgba8) -> u32 {
    [want.r, want.g, want.b]
        .iter()
        .zip(found)
        .map(|(a, b)| u32::from(a.abs_diff(b)))
        .sum()
}

fn page(w: f32, h: f32, bg: Rgba8, child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(w, h))
        .bg(bg)
        .child(child)
}

fn black_to_white() -> NodeBuilder {
    page(
        200.0,
        20.0,
        WHITE,
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 20.0))
            .bg(Background::radial(Radial::new().at(0, 50))
                .start(BLACK)
                .end(WHITE)),
    )
}

#[test]
fn gradient_midpoint_mixes_in_srgb_on_every_surface() {
    let mut failures = Vec::new();
    let half = 200.0 * std::f32::consts::SQRT_2 / 2.0;
    for surface in SURFACES {
        let mid = at(&shoot(black_to_white, 200, 20, surface), half, 10.0);
        println!("{surface:?}: black-to-white radial at t=0.5 is {mid:?}");
        if mid[0].abs_diff(128) > 10 {
            failures.push(format!(
                "{surface:?}: CSS mixes stops in sRGB, t=0.5 is ~128, got {mid:?}"
            ));
        }
    }
    check(&failures);
}

fn red_to_default() -> NodeBuilder {
    page(
        200.0,
        20.0,
        WHITE,
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 20.0))
            .bg(Background::linear(GradientAxis::LeftToRight).start(RED)),
    )
}

fn half_red() -> NodeBuilder {
    page(
        200.0,
        20.0,
        WHITE,
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 20.0))
            .bg(Rgba8 {
                r: 255,
                g: 0,
                b: 0,
                a: 128,
            }),
    )
}

#[test]
fn a_from_stop_fading_to_the_default_transparent_stop_keeps_its_hue() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let mid = at(&shoot(red_to_default, 200, 20, surface), 100.0, 10.0);
        let solid = at(&shoot(half_red, 200, 20, surface), 100.0, 10.0);
        println!(
            "{surface:?}: red-to-transparent midpoint {mid:?}, solid rgba(255,0,0,0.5) {solid:?}"
        );
        if off(
            mid,
            Rgba8 {
                r: solid[0],
                g: solid[1],
                b: solid[2],
                a: 255,
            },
        ) > 12
        {
            failures.push(format!(
                "{surface:?}: a premultiplied mix makes the midpoint rgba(255,0,0,0.5), drawn as {solid:?}, got {mid:?}"
            ));
        }
    }
    check(&failures);
}

fn gradient_over_bg_color() -> NodeBuilder {
    page(
        100.0,
        60.0,
        BLACK,
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 60.0))
            .bg(Background::radial(Radial::new()).start(RED).over(WHITE)),
    )
}

#[test]
fn a_background_color_shows_under_a_gradient_that_fades_out() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let corner = at(&shoot(gradient_over_bg_color, 100, 60, surface), 1.0, 1.0);
        if off(corner, WHITE) > 40 {
            failures.push(format!("{surface:?}: background-color paints under the gradient, corner is white, got {corner:?}"));
        }
    }
    check(&failures);
}

fn mean_ink(frame: &image::RgbaImage, columns: std::ops::Range<u32>) -> [f32; 3] {
    let (mut sum, mut count) = ([0.0f32; 3], 0.0f32);
    for y in 0..frame.height() {
        for x in columns.clone() {
            let p = frame.get_pixel(x, y);
            let rgb = [p[0], p[1], p[2]];
            if off(rgb, WHITE) > 300 {
                for (total, channel) in sum.iter_mut().zip(rgb) {
                    *total += f32::from(channel);
                }
                count += 1.0;
            }
        }
    }
    assert!(count > 20.0, "glyph pixels in {columns:?}");
    sum.map(|total| total / count)
}

fn word() -> TextBuilder {
    text("MMMMMM")
        .font_size(40.0, "test fixture")
        .font_weight(900)
        .text_color(BLACK)
}

fn container_clip_text() -> NodeBuilder {
    page(
        240.0,
        60.0,
        WHITE,
        container()
            .layout(LayoutStyle::block().fixed_size(240.0, 60.0))
            .bg(Background::linear(GradientAxis::LeftToRight)
                .start(RED)
                .end(BLUE))
            .bg_clip_text()
            .child(word()),
    )
}

#[test]
fn bg_clip_text_on_a_container_paints_its_text_children() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let frame = shoot(container_clip_text, 240, 60, surface);
        let left = mean_ink(&frame, 0..frame.width() / 5);
        if left[0] <= left[2] + 60.0 {
            failures.push(format!("{surface:?}: background-clip:text clips to descendant text, first letters red, got {left:?}"));
        }
    }
    check(&failures);
}

fn conic_word() -> NodeBuilder {
    page(
        240.0,
        60.0,
        WHITE,
        word()
            .bg(Background::conic(0).start(RED).middle(GREEN).end(BLUE))
            .bg_clip_text(),
    )
}

#[test]
fn conic_gradient_text_changes_color_across_the_word() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let frame = shoot(conic_word, 240, 60, surface);
        let width = frame.width();
        let left = mean_ink(&frame, 0..width / 5);
        let right = mean_ink(&frame, width * 3 / 5..width * 4 / 5);
        if !(right[0] > right[2] + 40.0 && left[2] > left[0] + 40.0) {
            failures.push(format!("{surface:?}: right half red-green, left half green-blue; left {left:?} right {right:?}"));
        }
    }
    check(&failures);
}

fn shifted(clip: bool) -> NodeBuilder {
    let node = container()
        .layout(LayoutStyle::block().fixed_size(60.0, 60.0))
        .bg(RED)
        .transform(Transform2D::IDENTITY.translate(-30.0, 0.0));
    let node = if clip {
        node.clip_path(ClipShape::Inset {
            inset: [0, 0, 0, 30],
            round: 0,
        })
    } else {
        node
    };
    page(60.0, 60.0, WHITE, node)
}

#[test]
fn a_clip_path_moves_with_its_node_transform() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let control = at(&shoot(|| shifted(false), 60, 60, surface), 15.0, 30.0);
        assert!(
            off(control, RED) <= 40,
            "{surface:?}: control, the translated box covers (15, 30), got {control:?}"
        );
        let probe = at(&shoot(|| shifted(true), 60, 60, surface), 15.0, 30.0);
        if off(probe, RED) > 40 {
            failures.push(format!("{surface:?}: clip-path is in local space, its right half lands at x 0..30, got {probe:?} at (15, 30)"));
        }
    }
    check(&failures);
}

fn near_card(degrees: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(300.0, 300.0)
                .padding_x(Space::raw(50.0, "test fixture"))
                .padding_y(Space::raw(100.0, "test fixture")),
        )
        .bg(WHITE)
        .perspective(80.0)
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(200.0, 100.0))
                .bg(RED)
                .rotate_y(degrees),
        )
}

#[test]
fn a_card_with_one_edge_behind_the_viewer_still_draws_its_centre() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let control = at(&shoot(|| near_card(30.0), 300, 300, surface), 150.0, 150.0);
        assert!(
            off(control, RED) <= 40,
            "{surface:?}: control, rotate_y(30) edge z 50 < 80 draws, got {control:?}"
        );
        let centre = at(&shoot(|| near_card(60.0), 300, 300, surface), 150.0, 150.0);
        if off(centre, RED) > 40 {
            failures.push(format!("{surface:?}: rotate_y(60) edge z 86.6 > perspective 80; centre (z 0, w 1) must draw, got {centre:?}"));
        }
    }
    check(&failures);
}

fn reversed_positions() -> NodeBuilder {
    page(
        100.0,
        20.0,
        WHITE,
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 20.0))
            .bg(Background::linear(GradientAxis::LeftToRight)
                .start(RED)
                .start_at(80)
                .end(BLUE)
                .end_at(20)),
    )
}

#[test]
fn a_stop_placed_before_an_earlier_stop_is_clamped_not_reordered() {
    let mut failures = Vec::new();
    for surface in SURFACES {
        let left = at(&shoot(reversed_positions, 100, 20, surface), 10.0, 10.0);
        if off(left, RED) > 40 {
            failures.push(format!(
                "{surface:?}: CSS clamps to-20% up to 80%, x=10 is red, got {left:?}"
            ));
        }
    }
    check(&failures);
}

#[path = "imaging_review/images.rs"]
mod images;
