#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::renderer::{DrawParams, ScreenSubmission, SlugTextPipeline};
use sigil::scene::{CommandScene, SceneBuilder, shape_glyph_run};
use sigil::style::{Rgba8, TextLineHeight, TextStyle};
use sigil::text::{OwnedTextEngine, TextConfig};
use sigil_kittest::gpu::{FORMAT, create_capture_texture, read_pixels, shared_device};

const BYTES_PER_PIXEL: u32 = 4;

const LOGICAL_W: f32 = 240.0;
const LOGICAL_H: f32 = 100.0;
const FONT_SIZE: f32 = 32.0;
const ORIGIN: (f32, f32) = (20.0, 20.0);
const TEXT: &str = "IIIIII";

const RAMP_THRESHOLD_PX: f32 = 1.75;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn glyph_edges_sharp_at_scale_1_0() {
    assert_direct_path_is_sharp(1.0);
}
#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn glyph_edges_sharp_at_scale_1_25() {
    assert_direct_path_is_sharp(1.25);
}
#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn glyph_edges_sharp_at_scale_1_5() {
    assert_direct_path_is_sharp(1.5);
}
#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn glyph_edges_sharp_at_scale_2_0() {
    assert_direct_path_is_sharp(2.0);
}

fn assert_direct_path_is_sharp(scale: f32) {
    let Some((device, queue)) = shared_device().expect("sigil-kittest").cloned() else {
        println!("no adapter — skipping glyph_pixel_check at scale {scale}");
        return;
    };
    let (pixels, w, h) = render_text(&device, &queue, scale, scale);
    assert_sharp(&format!("scale {scale}"), &pixels, w, h);
}

const EMBED_TEXTURE_SCALE: f32 = 6.0;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn embed_path_glyph_edges_are_sharp_with_matching_scale() {
    let (pixels, w, h) = render_embed_text(EMBED_TEXTURE_SCALE);
    assert_sharp(
        &format!("embed scale {EMBED_TEXTURE_SCALE} (matching submission scale)"),
        &pixels,
        w,
        h,
    );
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn embed_path_glyph_edges_are_soft_with_mismatched_scale() {
    let (pixels, w, h) = render_embed_text(1.0);
    assert_soft(
        &format!("embed scale 1.0 (mismatched — {EMBED_TEXTURE_SCALE}x physical target)"),
        &pixels,
        w,
        h,
    );
}

fn render_embed_text(submission_scale: f32) -> (Vec<u8>, u32, u32) {
    let mut renderer = sigil_kittest::gpu::headless_renderer()
        .expect("sigil-kittest")
        .expect("the glyph pixel check needs a GPU");
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();

    let phys_w = (LOGICAL_W * EMBED_TEXTURE_SCALE).round() as u32;
    let phys_h = (LOGICAL_H * EMBED_TEXTURE_SCALE).round() as u32;
    let texture = create_capture_texture(&device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let scene = build_text_scene();
    renderer
        .submit_screen(
            &ScreenSubmission {
                device: &device,
                queue: &queue,
                view: &view,
                format: FORMAT,
                viewport: (LOGICAL_W, LOGICAL_H),
                scale: submission_scale,
                clear: Some(sigil::style::Color::BLACK),
            },
            &scene,
        )
        .expect("submit_screen");

    let pixels = read_pixels(&device, &queue, &texture, phys_w, phys_h).expect("sigil-kittest");
    (pixels, phys_w, phys_h)
}

fn build_text_scene() -> CommandScene {
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: FONT_SIZE,
        font_weight: 400,
        line_height: TextLineHeight::FontSizeRelative(1.2),
        letter_spacing_em: 0.0,
        ..Default::default()
    };
    let rect = sigil::geometry::Rect::new(
        sigil::geometry::Point::new(ORIGIN.0, ORIGIN.1),
        LOGICAL_W - ORIGIN.0,
        LOGICAL_H - ORIGIN.1,
    );
    let run = shape_glyph_run(
        rect,
        TEXT.to_string(),
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        style,
        10_000,
    );
    let mut builder = SceneBuilder::new();
    builder.draw_glyph_run(run);
    builder.finish()
}

#[path = "renderer_glyph_pixel_check/polarity.rs"]
mod polarity;
#[path = "renderer_glyph_pixel_check/support.rs"]
mod support;

use support::*;
