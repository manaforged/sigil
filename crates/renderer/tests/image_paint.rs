//! Raster image wiring — a `NodeType::Image` uploads its pixels to a GPU
//! texture and samples them into its laid-out rect.
//!
//! Pins the full pipe: `ImageSource` → `NodeType::Image` → intrinsic
//! layout size → `SceneCommand::Image` → paint-time `register_image`
//! (once per content id) → Poly sprite sampling → real ink. The two-tone
//! fixture proves the texture is actually sampled, not filled with a
//! single average colour.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::cast_precision_loss)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_icons as _;
use sigil_shell as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;
use zeno as _;

#[path = "child_paint_repro/support.rs"]
#[allow(dead_code)]
mod support;

use sigil_kernel::{ImageSource, ObjectFit, container, image};
use sigil_layout::{LayoutBuilderExt, LayoutStyle, Overflow};
use sigil_renderer::{SceneCommand, emit_scene};
use support::{capture, close};

const SIZE: u32 = 32;

/// A `SIZE`×`SIZE` four-quadrant image: TL red, TR green, BL blue, BR
/// white. Asymmetric on both axes, so it fails a single-colour fallback,
/// a horizontal mirror, AND a vertical flip.
fn quadrant_image() -> ImageSource {
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = match (x < SIZE / 2, y < SIZE / 2) {
                (true, true) => [255, 0, 0, 255],
                (false, true) => [0, 255, 0, 255],
                (true, false) => [0, 0, 255, 255],
                (false, false) => [255, 255, 255, 255],
            };
            rgba.extend_from_slice(&px);
        }
    }
    ImageSource::from_rgba8(SIZE, SIZE, rgba).expect("well-formed RGBA")
}

#[test]
fn image_node_emits_an_image_command_carrying_its_pixels() {
    let mut cx = sigil_kernel::Cx::new_root();
    let root = cx.add(image(quadrant_image()));
    let engine_size = sigil_hal::Size::fixed(SIZE as f32, SIZE as f32);
    let mut engine = sigil_layout::LayoutEngine::with_text_measure(Box::new(support::StubMeasure));
    engine.compute(&mut cx, root, engine_size).expect("layout");

    let scene = emit_scene(&cx, root);
    let carried = scene
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::Image(draw) => Some(draw.source.clone()),
            _ => None,
        })
        .expect("image node must emit a DrawImage command");
    assert_eq!(carried.width(), SIZE);
    assert_eq!(carried.height(), SIZE);
}

#[test]
fn contain_letterboxes_a_wide_image_in_a_square_box() {
    // A 40x20 (2:1) solid-red image in a 32x32 box with Contain fits to
    // width (32 wide, 16 tall, centred), leaving clear strips top+bottom.
    let src = ImageSource::from_rgba8(40, 20, [255, 0, 0, 255].repeat(40 * 20)).unwrap();
    let img = capture(
        |cx| {
            let id = cx.add(image(src).fit(ObjectFit::Contain));
            if let Some(node) = cx.get_mut(id) {
                node.attach(LayoutStyle::block().fixed_size(SIZE as f32, SIZE as f32));
            }
            id
        },
        [SIZE, SIZE],
    );

    let center = img.get_pixel(SIZE / 2, SIZE / 2).0;
    let top_strip = img.get_pixel(SIZE / 2, 3).0;
    assert!(
        close(
            center[0],
            center[1],
            center[2],
            sigil_style::Rgba8 {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            },
            12
        ),
        "centre band must be the red image, got ({},{},{})",
        center[0],
        center[1],
        center[2],
    );
    assert!(
        top_strip[0] < 128,
        "letterbox strip must NOT be the red image (object-fit contain), got red={}",
        top_strip[0],
    );
}

#[test]
fn image_samples_its_texture_with_correct_orientation() {
    let img = capture(|cx| cx.add(image(quadrant_image())), [SIZE, SIZE]);
    let lo = SIZE / 4;
    let hi = 3 * SIZE / 4;
    // (sample point, expected rgb, label) — one per quadrant. A vertical
    // flip would swap top/bottom; a mirror would swap left/right.
    let checks = [
        ((lo, lo), [255, 0, 0], "top-left red"),
        ((hi, lo), [0, 255, 0], "top-right green"),
        ((lo, hi), [0, 0, 255], "bottom-left blue"),
        ((hi, hi), [255, 255, 255], "bottom-right white"),
    ];
    for ((x, y), [r, g, b], label) in checks {
        let px = img.get_pixel(x, y).0;
        assert!(
            close(
                px[0],
                px[1],
                px[2],
                sigil_style::Rgba8 { r, g, b, a: 255 },
                12
            ),
            "{label} quadrant at ({x},{y}) must sample ({r},{g},{b}), got ({},{},{})",
            px[0],
            px[1],
            px[2],
        );
    }
}

#[test]
fn twenty_four_distinct_images_all_paint() {
    // One ImageSource used to consume one of 16 atlas slots. Twenty-four
    // solid swatches must all sample after packing onto a shared page.
    let swatches: Vec<_> = (0..24_u8)
        .map(|i| {
            let color = [i.saturating_mul(10), 40, 200_u8.saturating_sub(i), 255];
            ImageSource::from_rgba8(8, 8, color.repeat(8 * 8)).expect("swatch")
        })
        .collect();
    let img = capture(
        |cx| {
            let mut row = container().layout(LayoutStyle::flex_row().fixed_size(192.0, 8.0));
            for source in &swatches {
                let source = source.clone();
                row = row.child(move |child_cx: &mut sigil_kernel::Cx| {
                    let id = child_cx.add(image(source));
                    if let Some(node) = child_cx.get_mut(id) {
                        node.attach(LayoutStyle::block().fixed_size(8.0, 8.0).shrink(0.0));
                    }
                    id
                });
            }
            cx.add(row)
        },
        [192, 8],
    );
    for i in 0..24 {
        let x = i * 8 + 4;
        let px = img.get_pixel(x, 4).0;
        // `i` is bounded by the 24 swatches in this test.
        #[allow(clippy::cast_possible_truncation)]
        let expect = (i as u8).saturating_mul(10);
        assert!(
            px[0].abs_diff(expect) < 16,
            "swatch {i} at x={x} must paint red≈{expect}, got {px:?}"
        );
    }
}

#[test]
fn overflow_hidden_scissors_image_pixels() {
    let source = ImageSource::from_rgba8(32, 32, [255, 0, 0, 255].repeat(32 * 32)).unwrap();
    let img = capture(
        |cx| {
            let child = move |child_cx: &mut sigil_kernel::Cx| {
                let id = child_cx.add(image(source));
                let _ = child_cx
                    .get_mut(id)
                    .unwrap()
                    .attach(LayoutStyle::block().fixed_size(32.0, 32.0));
                id
            };
            let clipped = container()
                .layout(
                    LayoutStyle::block()
                        .fixed_size(16.0, 16.0)
                        .overflow_x(Overflow::Hidden)
                        .overflow_y(Overflow::Hidden),
                )
                .child(child);
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(32.0, 32.0).padding(8.0))
                    .child(clipped),
            )
        },
        [32, 32],
    );

    let inside = img.get_pixel(12, 12).0;
    let outside = img.get_pixel(27, 12).0;
    assert!(
        inside[0] > 240 && inside[1] < 12 && inside[2] < 12,
        "{inside:?}"
    );
    assert!(
        outside[0] < 8 && outside[1] < 8 && outside[2] < 8,
        "{outside:?}"
    );
}
