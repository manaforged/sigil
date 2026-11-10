use sigil::geometry::PhysicalSize;
use sigil::prelude::*;
use sigil::renderer::CpuRenderer;
use sigil::style::Rgba;
use sigil::widgets::path;
use sigil_kittest::Harness;
use sigil_kittest::parity::{Frame, cpu_rgba};

use super::{INK, coverage, frame_of};

const VIEW: u32 = 64;
const OUTER: f32 = 10.0;
const INNER: f32 = 8.0;
const SQUARE: u32 = 10;
const LEFT: f32 = OUTER + INNER;

fn square_in_padded_box() -> impl IntoNode {
    let view = 2.0 * SQUARE as f32;
    let square = path(2 * SQUARE, 2 * SQUARE)
        .move_to(0.0, 0.0)
        .line_to(view, 0.0)
        .line_to(view, view)
        .line_to(0.0, view)
        .close()
        .fill(INK)
        .layout(LayoutStyle::block().fixed_size(SQUARE as f32, SQUARE as f32));
    container()
        .layout(LayoutStyle::flex_column().padding(Space::raw(OUTER, "test fixture")))
        .child(
            container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(30.0, 30.0)
                        .padding(Space::raw(INNER, "test fixture")),
                )
                .child(square),
        )
}

fn inside(x: u32, y: u32, scale: f32) -> bool {
    let logical = |at: u32| at as f32 / scale;
    let span = LEFT - 1.0..=LEFT + SQUARE as f32 + 1.0;
    span.contains(&logical(x)) && span.contains(&logical(y))
}

#[track_caller]
fn assert_placed(frame: &image::RgbaImage, scale: f32, what: &str) {
    let device = |at: f32| (at * scale) as u32;
    let centre = device(LEFT + SQUARE as f32 / 2.0);
    let lit = coverage(frame, centre, centre);
    let stray: Vec<(u32, u32)> = (0..frame.height())
        .flat_map(|y| (0..frame.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| !inside(x, y, scale) && coverage(frame, x, y) > 0.1)
        .collect();
    println!("{what}: centre coverage {lit:.2}, {} stray px", stray.len());
    assert!(
        lit > 0.9,
        "{what}: the square's centre ({LEFT}+5, {LEFT}+5) logical is unlit (coverage {lit:.2})"
    );
    assert!(
        stray.is_empty(),
        "{what}: {} px lit outside the square at {LEFT}..{} logical, first {:?}",
        stray.len(),
        LEFT + SQUARE as f32,
        stray.first()
    );
}

#[test]
fn a_resized_path_in_a_padded_offset_box_lands_in_the_box_on_pixels() {
    for scale in [1.0, 2.0] {
        let mut ui = Harness::new(square_in_padded_box)
            .unaudited()
            .pixels(VIEW, VIEW)
            .scale(scale);
        assert_placed(&frame_of(&mut ui), scale, &format!("pixels {scale}x"));
    }
}

#[test]
fn a_resized_path_in_a_padded_offset_box_lands_in_the_box_on_the_gpu() {
    for scale in [1.0, 2.0] {
        let mut ui = Harness::new(square_in_padded_box)
            .unaudited()
            .gpu(VIEW, VIEW, scale);
        assert_placed(&frame_of(&mut ui), scale, &format!("gpu {scale}x"));
    }
}

#[test]
fn a_resized_path_in_a_padded_offset_box_lands_in_the_box_on_the_cpu() {
    for scale in [1.0, 2.0] {
        let mut ui = Harness::new(square_in_padded_box)
            .unaudited()
            .pixels(VIEW, VIEW);
        let viewport = (VIEW as f32, VIEW as f32);
        let scene = ui.host().emit(viewport);
        let side = (VIEW as f32 * scale) as u32;
        let frame = Frame {
            scene: &scene,
            viewport,
            size: PhysicalSize::new(side, side),
            clear: Rgba::new(0.0, 0.0, 0.0, 1.0),
        };
        let rgba = cpu_rgba(&mut CpuRenderer::new(), &frame).expect("cpu render");
        let image = image::RgbaImage::from_raw(side, side, rgba).expect("cpu frame size");
        assert_placed(&image, scale, &format!("cpu {scale}x"));
    }
}
