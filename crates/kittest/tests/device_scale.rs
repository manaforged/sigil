#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::print_stdout, reason = "the parity check prints its measure")]
#![allow(clippy::panic, reason = "a test fails loudly on a missing value")]
#![allow(clippy::cast_precision_loss, reason = "pixel sides are small")]

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::kernel::ImageSource;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{
    CornerRadii, DrawGradient, DrawRect, GradientStop, LinearGradient, SceneBuilder,
};
use sigil::style::Rgba;
use sigil::widgets::canvas;
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, MAX_MEAN_DIFF, MIN_CLOSE_FRACTION, cpu_rgba, diff};

const WIDTH: u32 = 120;
const HEIGHT: u32 = 60;
const SCALE: f32 = 2.0;
const DEVICE: (u32, u32) = (WIDTH * 2, HEIGHT * 2);
const HAIRLINE_ROW: u32 = 20;
const INK: Color = Color {
    r: 255,
    g: 40,
    b: 40,
    a: 255,
};

fn hairline(scene: &mut SceneBuilder, rect: Rect) {
    let at = Point::new(rect.origin.x + 10.5, rect.origin.y + 2.0);
    scene.draw_rect(DrawRect::new(Rect::new(at, 1.0, 16.0), INK));
}

fn hairline_and_text() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_column())
        .child(canvas(hairline).width(40.0).height(20.0))
        .child(text("Sigil device scale"))
}

fn shapes(scene: &mut SceneBuilder, rect: Rect) {
    let card = Rect::new(rect.origin, 32.0, 32.0);
    let rounded = DrawRect::new(
        card,
        Color {
            r: 240,
            g: 200,
            b: 60,
            a: 255,
        },
    );
    scene.draw_rect(rounded.border_radius(CornerRadii::uniform(8.0)));
    let rect = Rect::new(Point::new(rect.origin.x + 36.0, rect.origin.y), 24.0, 32.0);
    let end = Point::new(rect.origin.x + rect.width, rect.origin.y);
    let stops = vec![
        GradientStop::new(0.0, INK),
        GradientStop::new(
            1.0,
            Color {
                r: 20,
                g: 90,
                b: 255,
                a: 255,
            },
        ),
    ];
    let paint = LinearGradient::new(rect.origin, end, stops);
    scene.draw_gradient(DrawGradient::new(rect, paint));
}

fn checker() -> ImageSource {
    let rgba = (0..64_u32)
        .flat_map(|at| {
            if (at / 8 + at % 8) % 2 == 0 {
                [250, 250, 250, 255]
            } else {
                [10, 120, 40, 255]
            }
        })
        .collect();
    ImageSource::from_rgba8(8, 8, rgba).expect("8x8 rgba")
}

fn fixed_scene() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_row())
        .child(text("Sigil"))
        .child(canvas(shapes).width(60.0).height(32.0))
        .child(sigil::kernel::image(checker()).width(16.0).height(16.0))
}

fn distance(a: image::Rgba<u8>, b: image::Rgba<u8>) -> u32 {
    a.0.iter()
        .zip(b.0)
        .take(3)
        .map(|(x, y)| u32::from(x.abs_diff(y)))
        .sum()
}

fn hairline_columns(frame: &image::RgbaImage) -> (usize, usize) {
    let background = *frame.get_pixel(0, HAIRLINE_ROW);
    let full = distance(image::Rgba([INK.r, INK.g, INK.b, INK.a]), background);
    let row = (0..frame.width()).map(|x| distance(*frame.get_pixel(x, HAIRLINE_ROW), background));
    let (mut lit, mut partial) = (0, 0);
    for contrast in row {
        if contrast * 10 >= full * 9 {
            lit += 1;
        } else if contrast * 10 > full {
            partial += 1;
        }
    }
    (lit, partial)
}

fn split_blocks(frame: &image::RgbaImage, rows: std::ops::Range<u32>) -> usize {
    let block = |x: u32, y: u32| {
        let first = *frame.get_pixel(x, y);
        [(1, 0), (0, 1), (1, 1)]
            .iter()
            .any(|(dx, dy)| distance(*frame.get_pixel(x + dx, y + dy), first) > 24)
    };
    rows.step_by(2)
        .flat_map(|y| (0..frame.width() - 1).step_by(2).map(move |x| (x, y)))
        .filter(|(x, y)| block(*x, *y))
        .count()
}

#[track_caller]
#[test]
fn a_pixel_render_at_scale_two_resolves_hairlines_and_text_at_device_pixels() {
    let mut ui = Harness::new(hairline_and_text)
        .pixels(WIDTH, HEIGHT)
        .scale(SCALE);
    let frame = ui.expect_image();
    assert_eq!(
        frame.dimensions(),
        DEVICE,
        "image is the logical size times the scale"
    );
    let (lit, partial) = hairline_columns(&frame);
    assert_eq!(
        (lit, partial),
        (2, 0),
        "a 1-logical-px hairline covers 2 device px with no blurred edge"
    );
    let split = split_blocks(&frame, 40..DEVICE.1 - 1);
    assert!(
        split >= 20,
        "text edges fall inside 2x2 device blocks ({split} split blocks); an upscaled 1x frame has none"
    );
}

#[test]
fn the_gpu_surface_returns_an_image_of_its_frame() {
    let mut ui = Harness::new(hairline_and_text).gpu(WIDTH, HEIGHT, SCALE);
    let frame = ui.expect_image();
    assert_eq!(
        frame.dimensions(),
        DEVICE,
        "the GPU frame is the logical size times the scale"
    );
    assert_eq!(
        hairline_columns(&frame),
        (2, 0),
        "the GPU frame shows the hairline at device pixels"
    );
}

#[test]
fn one_scene_agrees_on_the_cpu_and_the_gpu() {
    let mut ui = Harness::new(fixed_scene).gpu(WIDTH, HEIGHT, SCALE);
    let gpu = ui.expect_image();
    let viewport = (WIDTH as f32, HEIGHT as f32);
    let scene = ui.host().emit(viewport);
    let frame = Frame {
        scene: &scene,
        viewport,
        size: PhysicalSize::new(DEVICE.0, DEVICE.1),
        clear: Rgba::new(0.0, 0.0, 0.0, 1.0),
    };
    let cpu = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    let drawn = gpu.pixels().filter(|px| px.0[..3] != [0, 0, 0]).count();
    assert!(
        drawn * 10 > gpu.pixels().len(),
        "the scene draws ink ({drawn} pixels)"
    );
    let measured = diff(gpu.as_raw(), &cpu);
    println!(
        "cpu/gpu: mean {:.2}/255, {:.1}% close",
        measured.mean,
        measured.close * 100.0
    );
    assert!(
        measured.mean <= MAX_MEAN_DIFF && measured.close >= MIN_CLOSE_FRACTION,
        "cpu and gpu differ: {measured:?}, limits mean {MAX_MEAN_DIFF} close {MIN_CLOSE_FRACTION}"
    );
}
