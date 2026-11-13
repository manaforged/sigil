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
    reason = "pixel coordinates are small"
)]

use sigil::geometry::PhysicalSize;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::style::{Background, GradientAxis, Radial, RadialExtent, Rgba};
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, cpu_rgba, diff};

#[path = "paints/borders.rs"]
mod borders;
#[path = "paints/route.rs"]
mod route;

pub(crate) const SCALE: f32 = 2.0;
const RED: Color = rgb(255, 0, 0);
const GREEN: Color = rgb(0, 255, 0);
const BLUE: Color = rgb(0, 0, 255);
pub(crate) const WHITE: Color = rgb(255, 255, 255);
pub(crate) const INK: Color = rgb(0, 0, 0);

pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b, a: 255 }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Surface {
    Pixels,
    Gpu,
    Cpu,
}

pub(crate) const SURFACES: [Surface; 3] = [Surface::Pixels, Surface::Gpu, Surface::Cpu];

pub(crate) struct Shot {
    pub(crate) image: image::RgbaImage,
    pub(crate) surface: Surface,
}

impl Shot {
    pub(crate) fn at(&self, x: f32, y: f32) -> [u8; 3] {
        let pixel = self.image.get_pixel((x * SCALE) as u32, (y * SCALE) as u32);
        [pixel[0], pixel[1], pixel[2]]
    }
}

pub(crate) fn page(width: f32, height: f32, child: impl IntoNode + 'static) -> impl IntoNode {
    container()
        .layout(LayoutStyle::block().fixed_size(width, height))
        .background(WHITE)
        .child(child)
}

pub(crate) fn shoot<A: IntoNode + 'static>(
    app: fn() -> A,
    size: (u32, u32),
    surface: Surface,
) -> Shot {
    let (width, height) = size;
    let image = match surface {
        Surface::Pixels => Harness::new(app)
            .pixels(width, height)
            .scale(SCALE)
            .expect_image(),
        Surface::Gpu => Harness::new(app)
            .gpu(width, height)
            .scale(SCALE)
            .expect_image(),
        Surface::Cpu => cpu_frame(
            &mut Harness::new(app).pixels(width, height).scale(SCALE),
            size,
        ),
    };
    Shot { image, surface }
}

fn cpu_frame(ui: &mut Harness, (width, height): (u32, u32)) -> image::RgbaImage {
    let viewport = (width as f32, height as f32);
    let scene = ui.host().emit(viewport);
    let size = PhysicalSize::new(
        (width as f32 * SCALE) as u32,
        (height as f32 * SCALE) as u32,
    );
    let frame = Frame {
        scene: &scene,
        viewport,
        size,
        clear: Rgba::new(1.0, 1.0, 1.0, 1.0),
    };
    let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    image::RgbaImage::from_raw(size.width, size.height, rgba).expect("cpu frame size")
}

pub(crate) fn distance(a: [u8; 3], b: Color) -> u32 {
    [b.r, b.g, b.b]
        .iter()
        .zip(a)
        .map(|(x, y)| u32::from(x.abs_diff(y)))
        .sum()
}

#[track_caller]
fn assert_near(shot: &Shot, point: (f32, f32), want: Color, limit: u32, what: &str) {
    let found = shot.at(point.0, point.1);
    let off = distance(found, want);
    assert!(
        off <= limit,
        "{:?}: {what} at {point:?} is {found:?}, want {want:?} within {limit} (off {off})",
        shot.surface
    );
}

fn radial_box() -> impl IntoNode {
    page(
        100.0,
        60.0,
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 60.0))
            .background(Background::radial(Radial::new()).start(RED).end(BLUE)),
    )
}

#[test]
fn a_radial_gradient_is_the_from_color_at_its_centre_and_the_to_color_at_the_corner() {
    for surface in SURFACES {
        let shot = shoot(radial_box, (100, 60), surface);
        assert_near(&shot, (50.0, 30.0), RED, 24, "the centre");
        assert_near(
            &shot,
            (0.5, 0.5),
            BLUE,
            40,
            "the corner of the farthest-corner ellipse",
        );
        let edge = shot.at(99.5, 30.0);
        assert!(
            edge[0] > 20 && edge[2] > 20,
            "{surface:?}: the right edge midpoint sits at 69% of the ellipse and mixes both colors, got {edge:?}"
        );
    }
}

fn closest_side_circle() -> impl IntoNode {
    page(
        100.0,
        60.0,
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 60.0))
            .background(
                Background::radial(
                    Radial::new()
                        .circle(true)
                        .extent(RadialExtent::ClosestSide)
                        .at(25, 50),
                )
                .start(RED)
                .end(BLUE),
            ),
    )
}

#[test]
fn a_closest_side_circle_reaches_the_to_color_at_the_nearest_side() {
    for surface in SURFACES {
        let shot = shoot(closest_side_circle, (100, 60), surface);
        assert_near(&shot, (25.0, 30.0), RED, 60, "the centre at 25% 50%");
        assert_near(
            &shot,
            (25.0 + 31.0, 30.0),
            BLUE,
            24,
            "just past the 25 px radius",
        );
        assert_near(&shot, (25.0, 0.5), BLUE, 40, "the top side, 30 px away");
    }
}

fn conic_box(from: i16) -> impl IntoNode {
    page(
        80.0,
        80.0,
        container()
            .layout(LayoutStyle::block().fixed_size(80.0, 80.0))
            .background(Background::conic(from).start(RED).middle(GREEN).end(BLUE)),
    )
}

fn dominant(pixel: [u8; 3]) -> usize {
    (0..3).max_by_key(|&channel| pixel[channel]).unwrap_or(0)
}

#[test]
fn a_conic_gradient_turns_clockwise_from_the_top() {
    for surface in SURFACES {
        let shot = shoot(|| conic_box(0), (80, 80), surface);
        let probe = |degrees: f32| {
            let angle = degrees.to_radians();
            shot.at(
                30.0f32.mul_add(angle.sin(), 40.0),
                30.0f32.mul_add(-angle.cos(), 40.0),
            )
        };
        let [top, right, bottom, left] = [3.0, 90.0, 180.0, 270.0].map(probe);
        assert_eq!(
            dominant(top),
            0,
            "{surface:?}: just past 0° is red, got {top:?}"
        );
        assert_eq!(
            dominant(bottom),
            1,
            "{surface:?}: 180° is green, got {bottom:?}"
        );
        assert!(
            right[2] < 16 && right[0] > 40 && right[1] > 40,
            "{surface:?}: 90° mixes red and green only, got {right:?}"
        );
        assert!(
            left[0] < 16 && left[1] > 40 && left[2] > 40,
            "{surface:?}: 270° mixes green and blue only, got {left:?}"
        );
    }
}

#[test]
fn a_conic_from_angle_rotates_the_start() {
    for surface in SURFACES {
        let shot = shoot(|| conic_box(90), (80, 80), surface);
        let just_below_right = shot.at(70.0, 41.5);
        assert_eq!(
            dominant(just_below_right),
            0,
            "{surface:?}: from 90° starts red at 3 o'clock, got {just_below_right:?}"
        );
    }
}

fn gradient_word() -> impl IntoNode {
    page(
        240.0,
        60.0,
        text("MMMMMM")
            .font_size(40.0, "test fixture")
            .font_weight(900)
            .text_color(INK)
            .background(
                Background::linear(GradientAxis::LeftToRight)
                    .start(RED)
                    .end(BLUE),
            )
            .background_clip_text(),
    )
}

fn ink_mean(shot: &Shot, columns: std::ops::Range<u32>) -> [f32; 3] {
    let mut sum = [0.0f32; 3];
    let mut count = 0.0f32;
    for y in 0..shot.image.height() {
        for x in columns.clone() {
            let pixel = shot.image.get_pixel(x, y);
            let rgb = [pixel[0], pixel[1], pixel[2]];
            if distance(rgb, WHITE) > 300 {
                for (total, channel) in sum.iter_mut().zip(rgb) {
                    *total += f32::from(channel);
                }
                count += 1.0;
            }
        }
    }
    assert!(
        count > 20.0,
        "{:?}: the text has glyph pixels in {columns:?}",
        shot.surface
    );
    sum.map(|total| total / count)
}

#[test]
fn gradient_text_glyphs_change_color_across_the_word() {
    for surface in SURFACES {
        let shot = shoot(gradient_word, (240, 60), surface);
        let width = shot.image.width();
        let left = ink_mean(&shot, 0..width / 5);
        let right = ink_mean(&shot, width * 3 / 5..width * 4 / 5);
        println!("{surface:?}: left ink {left:?}, right ink {right:?}");
        assert!(
            left[0] > left[2] + 60.0,
            "{surface:?}: the first letters are red, not black ink: {left:?}"
        );
        assert!(
            right[2] > right[0],
            "{surface:?}: the later letters turn blue: {right:?}"
        );
    }
}

fn quarters() -> ImageSource {
    let mut rgba = Vec::new();
    for _y in 0..10 {
        for x in 0..10 {
            let color = if x < 5 { RED } else { BLUE };
            rgba.extend_from_slice(&[color.r, color.g, color.b, 255]);
        }
    }
    ImageSource::from_rgba8(10, 10, rgba).expect("10x10 image")
}

fn tiled() -> impl IntoNode {
    page(
        40.0,
        20.0,
        container()
            .layout(LayoutStyle::block().fixed_size(35.0, 20.0))
            .background_image(quarters()),
    )
}

#[test]
fn an_image_paint_tiles_at_its_natural_size() {
    for surface in SURFACES {
        let shot = shoot(tiled, (40, 20), surface);
        for (x, want) in [
            (2.5, RED),
            (7.5, BLUE),
            (12.5, RED),
            (17.5, BLUE),
            (32.5, RED),
        ] {
            assert_near(&shot, (x, 5.0), want, 40, "a first-row tile");
            assert_near(&shot, (x, 15.0), want, 40, "a second-row tile");
        }
        assert_near(&shot, (37.5, 5.0), WHITE, 24, "past the 35 px box");
    }
}

fn parity_scene() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_row().fixed_size(400.0, 80.0))
        .background(WHITE)
        .child(radial_box())
        .child(conic_box(45))
        .child(borders::dashed_card())
}

#[test]
fn paints_agree_on_the_cpu_and_the_gpu() {
    let gpu = shoot(parity_scene, (400, 80), Surface::Gpu);
    let cpu = shoot(parity_scene, (400, 80), Surface::Cpu);
    let measured = diff(gpu.image.as_raw(), cpu.image.as_raw());
    println!(
        "cpu/gpu: mean {:.2}/255, {:.1}% close",
        measured.mean,
        measured.close * 100.0
    );
    measured.assert_close();
}
