#![allow(clippy::print_stdout, reason = "the checks print their measures")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel coordinates and channels are small"
)]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]

use sigil::geometry::{PhysicalSize, Point, Rect};
use sigil::kernel::container;
use sigil::layout::LayoutStyle;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::scene::{
    BlendMode, CommandScene, CornerRadii, DrawRect, LayerEffect, LayerScope, SceneBuilder,
};
use sigil::style::Rgba;
use sigil::widgets::canvas;
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, cpu_rgba, diff};

#[path = "blend/model.rs"]
mod model;
#[path = "blend/perf.rs"]
mod perf;
#[path = "blend/style.rs"]
mod style;

const CELL: f32 = 20.0;
const SCALE: f32 = 2.0;
const COLUMNS: usize = BlendMode::ALL.len();
const WIDTH: u32 = (CELL as u32) * COLUMNS as u32;
const HEIGHT: u32 = (CELL as u32) * 2;
const BACKDROP: [u8; 3] = [200, 100, 50];
const SOURCE: [u8; 3] = [50, 150, 250];
const HALF: u8 = 128;
const TOLERANCE: u8 = 2;

const fn rgba([r, g, b]: [u8; 3], a: u8) -> Color {
    Color { r, g, b, a }
}

fn blended(scene: &mut SceneBuilder, rect: Rect, mode: BlendMode, color: Color) {
    let scope = LayerScope {
        clip: rect,
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Identity,
        opacity: 1.0,
        blend: mode,
    };
    scene.composite(scope, |scene| scene.draw_rect(DrawRect::new(rect, color)));
}

fn cell(mode: BlendMode, alpha: u8) -> impl IntoNode {
    let paint = canvas(move |scene: &mut SceneBuilder, rect: Rect| {
        scene.draw_rect(DrawRect::new(rect, rgba(BACKDROP, 255)));
        let inner = Rect::new(
            Point::new(rect.origin.x + 2.0, rect.origin.y + 2.0),
            rect.width - 4.0,
            rect.height - 4.0,
        );
        blended(scene, inner, mode, rgba(SOURCE, alpha));
    })
    .width(CELL)
    .height(CELL);
    container()
        .layout(LayoutStyle::block().fixed_size(CELL, CELL))
        .child(paint)
}

fn row(alpha: u8) -> impl IntoNode {
    BlendMode::ALL.into_iter().fold(
        container().layout(LayoutStyle::flex_row().fixed_size(WIDTH as f32, CELL)),
        |row, mode| row.child(cell(mode, alpha)),
    )
}

fn gallery() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_column().fixed_size(WIDTH as f32, HEIGHT as f32))
        .child(row(255))
        .child(row(HALF))
}

fn cpu_frame(scene: &CommandScene) -> image::RgbaImage {
    let (device_w, device_h) = (
        (WIDTH as f32 * SCALE) as u32,
        (HEIGHT as f32 * SCALE) as u32,
    );
    let frame = Frame {
        scene,
        viewport: (WIDTH as f32, HEIGHT as f32),
        size: PhysicalSize::new(device_w, device_h),
        clear: Rgba::new(0.0, 0.0, 0.0, 1.0),
    };
    let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
    image::RgbaImage::from_raw(device_w, device_h, rgba).expect("cpu frame")
}

fn frames() -> Vec<(&'static str, image::RgbaImage)> {
    let mut pixels = Harness::new(gallery).pixels(WIDTH, HEIGHT).scale(SCALE);
    let mut surface = Harness::new(gallery).gpu(WIDTH, HEIGHT).scale(SCALE);
    let scene = surface.host().emit((WIDTH as f32, HEIGHT as f32));
    vec![
        ("pixels", pixels.expect_image()),
        ("gpu", surface.expect_image()),
        ("cpu", cpu_frame(&scene)),
    ]
}

fn probe(frame: &image::RgbaImage, column: usize, row: usize) -> [u8; 3] {
    let left = (column as f32 + 0.5) * CELL * SCALE;
    let top = (row as f32 + 0.5) * CELL * SCALE;
    let [red, green, blue, _] = frame.get_pixel(left as u32, top as u32).0;
    [red, green, blue]
}

fn checked(mode: BlendMode, alpha: u8) -> bool {
    alpha == 255 || mode != BlendMode::Normal
}

#[track_caller]
fn assert_modes(row: usize, alpha: u8) {
    let mut failures = Vec::new();
    for (name, frame) in frames() {
        for (column, mode) in BlendMode::ALL.into_iter().enumerate() {
            if !checked(mode, alpha) {
                continue;
            }
            let expected = model::composite(mode, BACKDROP, SOURCE, alpha);
            let found = probe(&frame, column, row);
            let close = found
                .iter()
                .zip(expected)
                .all(|(a, b)| a.abs_diff(b) <= TOLERANCE);
            println!("{name} {mode:?} alpha {alpha}: found {found:?} expected {expected:?}");
            if !close {
                failures.push(format!("{name} {mode:?}: {found:?} != {expected:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "pixels off the spec formula by more than {TOLERANCE}/255: {failures:#?}"
    );
}

#[test]
fn each_blend_mode_yields_the_spec_pixel_for_an_opaque_source() {
    assert_modes(0, 255);
}

#[test]
fn a_half_transparent_source_composites_its_blend_over_the_backdrop() {
    assert_modes(1, HALF);
}

#[test]
fn blended_layers_agree_on_the_cpu_and_the_gpu() {
    let mut ui = Harness::new(gallery).gpu(WIDTH, HEIGHT).scale(SCALE);
    let gpu = ui.expect_image();
    let scene = ui.host().emit((WIDTH as f32, HEIGHT as f32));
    let cpu = cpu_frame(&scene);
    let measured = diff(gpu.as_raw(), cpu.as_raw());
    println!(
        "blend cpu/gpu: mean {:.2}/255, {:.1}% close",
        measured.mean,
        measured.close * 100.0
    );
    measured.assert_close();
    for (row, alpha) in [(0, 255), (1, HALF)] {
        for column in (0..COLUMNS).filter(|c| checked(BlendMode::ALL[*c], alpha)) {
            let (a, b) = (probe(&gpu, column, row), probe(&cpu, column, row));
            let gap = a.iter().zip(b).map(|(x, y)| x.abs_diff(y)).max();
            assert!(
                gap <= Some(TOLERANCE * 2),
                "{:?} row {row}: gpu {a:?} cpu {b:?}",
                BlendMode::ALL[column]
            );
        }
    }
}
