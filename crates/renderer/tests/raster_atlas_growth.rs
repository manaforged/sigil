//! Tiny-atlas growth-boundary and atlas-full end-to-end checks — Wave-3 D3
//! (`docs/plans/2026-07-07-text-quality-and-gap-closure.md` Wave 3, DR-055
//! §2/§3). Uses the `#[doc(hidden)]` `set_test_atlas_size_override` hook to
//! force the real 1024²→2048² atlas down to a few dozen texels, so a small
//! paragraph — not the ~10k distinct glyphs a real 2048² atlas would need —
//! can cross a growth boundary mid-accumulation, or overflow it entirely.
//!
//! Both tests render through the real embed path (`shape_glyph_run` →
//! `submit_screen`), the same production code the `draw_glyph_run` /
//! `RasterTextPipeline::place_glyphs` growth-and-fallback logic runs
//! through — this validates the `normalize_uvs`-after-growth fix (and the
//! atlas-full → vector-fallback leftover path) end-to-end, not just via the
//! unit tests already in `raster_paint.rs`.
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::print_stdout)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;
use zeno as _;

use std::sync::mpsc;

use sigil_renderer::{
    CommandScene, SceneBuilder, ScreenSubmission, TextTuning, WgpuRenderer,
    set_test_atlas_size_override, set_text_tuning, shape_glyph_run,
};
use sigil_style::{Rgba8, TextLineHeight, TextStyle};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

// Wide enough that each fixture's lines lay out without soft-wrapping
// (explicit `\n` controls line breaks) and tall enough to hold the
// fixtures' 3 lines with a small margin.
const LOGICAL_W: f32 = 460.0;
const LOGICAL_H: f32 = 90.0;
const FONT_SIZE: f32 = 14.0; // within RASTER_MAX_LOGICAL_PX (20px)
const ORIGIN: (f32, f32) = (8.0, 12.0);

/// Restores both process globals this file mutates — [`TextTuning`] and the
/// atlas size override — on drop, including on panic. Both are shared
/// state other tests in the same `--test-threads=1` run depend on seeing
/// at their production defaults.
struct TestGuard;
impl Drop for TestGuard {
    fn drop(&mut self) {
        set_text_tuning(TextTuning::default());
        set_test_atlas_size_override(0, 0);
    }
}

/// ~30 unique glyphs (upper/lower case, digits, punctuation) — enough that
/// a 64² atlas (holding roughly two dozen ~10×14px hinted masks) fills and
/// grows to 128² partway through a single accumulation, then the repeated
/// second/third copies exercise the cache under the grown size.
const GROWTH_TEXT: &str = "The Quick Brown Fox Jumps 0123456789!?.,\n\
                            The Quick Brown Fox Jumps 0123456789!?.,\n\
                            The Quick Brown Fox Jumps 0123456789!?.,";

/// Wave-3 D3 (DR-055 §2 kill-criteria follow-up): force a growth crossing
/// (64² → 128²) mid-accumulation with [`GROWTH_TEXT`], then compare the
/// PIXELS against a reference render of the same content through the same
/// code path with the real (uncapped) atlas sizes — where no grow ever
/// happens, since ~30 unique glyphs is far below 1024²'s capacity. A
/// `normalize_uvs` regression (normalizing against the pre-growth size,
/// the actual review-caught bug) corrupts UVs for every instance placed
/// before the grow — sampling out-of-range atlas texels, which
/// `ClampToEdge` turns into wrong-but-nonzero ink, not blank pixels, so a
/// mere "is there ink" check would NOT catch it. A pixel comparison does.
#[test]
fn growth_crossing_produces_correctly_placed_ink() {
    let _guard = TestGuard;
    let Some(mut ref_renderer) = try_headless_renderer() else {
        println!("no adapter — skipping growth_crossing_produces_correctly_placed_ink");
        return;
    };
    let Some(mut tiny_renderer) = try_headless_renderer() else {
        println!("no adapter — skipping growth_crossing_produces_correctly_placed_ink");
        return;
    };
    set_text_tuning(TextTuning {
        raster: true,
        ..TextTuning::default()
    });

    set_test_atlas_size_override(0, 0); // real defaults — no grow needed
    let (reference, rw, rh) = render(&mut ref_renderer, GROWTH_TEXT);

    set_test_atlas_size_override(40, 2048); // grows once (40→80), never full
    let (tiny, tw, th) = render(&mut tiny_renderer, GROWTH_TEXT);

    assert_eq!((rw, rh), (tw, th), "viewport size mismatch between renders");
    let diff = pixel_diff_fraction(&reference, &tiny);
    println!("growth-crossing: pixel_diff_fraction={diff:.6} (0.0 == identical to reference)");
    assert!(
        diff < 0.01,
        "tiny-atlas-forced-growth render diverged from the real-atlas reference by \
         {diff:.4} of pixels — normalize_uvs-after-growth regression?"
    );
}

/// ~90 unique glyphs (full mixed-case alphabet, digits, wide punctuation
/// set, one per line) — enough to overflow a 32²→64² atlas after its one
/// permitted grow, forcing `RasterTextPipeline::record_paragraph` to fall
/// back the overflow glyphs to vector Slug (DR-055 §2's "atlas full ->
/// silent vector fallback"). Asserts the MIXED raster+vector render still
/// carries ink in both the early (raster-placed) and late (vector-
/// fallback) regions — not just a nonzero total, which a render that
/// silently dropped the fallback half would still pass.
const OVERFLOW_UNIQUE_GLYPHS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ\n\
                                       abcdefghijklmnopqrstuvwxyz\n\
                                       0123456789!?.,;:()[]{}+-*/=<>@#$%^&";

/// Number of horizontal bands [`OVERFLOW_UNIQUE_GLYPHS`]' 3 explicit lines
/// are checked in — one per line, so a whole dropped line (not just a
/// dimmer one) is caught regardless of which line the atlas happened to
/// fill up on.
const OVERFLOW_BANDS: u32 = 3;

/// Wave-3 D3 (DR-055 §2 "atlas full -> silent vector fallback"): force an
/// overflow (32²→64², then full) with [`OVERFLOW_UNIQUE_GLYPHS`] (~90
/// unique glyphs, one per line across 3 lines) and compare each line's ink
/// against a PURE VECTOR reference render of the same content
/// (`TextTuning.raster = false`, which never drops a glyph). A render that
/// silently swallowed the atlas-full leftover instead of handing it to
/// vector Slug would show near-zero ink in whichever band the overflow
/// landed on, while the vector reference always has ink in every band —
/// catching a dropped line regardless of where in the sequence it falls.
#[test]
fn atlas_full_falls_back_to_vector_with_ink_throughout() {
    let _guard = TestGuard;
    let Some(mut vector_renderer) = try_headless_renderer() else {
        println!("no adapter — skipping atlas_full_falls_back_to_vector_with_ink_throughout");
        return;
    };
    let Some(mut mixed_renderer) = try_headless_renderer() else {
        println!("no adapter — skipping atlas_full_falls_back_to_vector_with_ink_throughout");
        return;
    };

    set_test_atlas_size_override(0, 0);
    set_text_tuning(TextTuning {
        raster: false,
        ..TextTuning::default()
    });
    let (vector_px, vw, vh) = render(&mut vector_renderer, OVERFLOW_UNIQUE_GLYPHS);

    set_test_atlas_size_override(32, 64); // grows once to 64, then overflows
    set_text_tuning(TextTuning {
        raster: true,
        ..TextTuning::default()
    });
    let (mixed_px, mw, mh) = render(&mut mixed_renderer, OVERFLOW_UNIQUE_GLYPHS);
    assert_eq!((vw, vh), (mw, mh), "viewport size mismatch between renders");

    // Band the ACTUAL ink extent of the vector reference (its 3 lines don't
    // divide the padded viewport into equal thirds), not the raw viewport —
    // otherwise a band can straddle a near-empty gap between lines and read
    // as "no ink" in both renders, proving nothing.
    let (ink_y0, ink_y1) =
        ink_row_bounds(&vector_px, vw, vh).expect("vector reference produced no ink at all");
    for band in 0..OVERFLOW_BANDS {
        let span = ink_y1 - ink_y0;
        let y0 = ink_y0 + span * band / OVERFLOW_BANDS;
        let y1 = ink_y0 + span * (band + 1) / OVERFLOW_BANDS;
        let vector_ink = ink_fraction_band(&vector_px, vw, vh, y0, y1);
        let mixed_ink = ink_fraction_band(&mixed_px, mw, mh, y0, y1);
        println!(
            "atlas-full band {band} (rows {y0}..{y1}): vector_ink={vector_ink:.4} \
             mixed_ink={mixed_ink:.4}"
        );
        assert!(
            vector_ink > 0.0,
            "band {band}: vector reference itself has no ink — fixture/layout bug, not the \
             thing under test"
        );
        assert!(
            mixed_ink > 0.2 * vector_ink,
            "band {band}: mixed raster+vector render has {mixed_ink:.4} ink vs. vector \
             reference's {vector_ink:.4} — did the atlas-full leftover fail to reach vector \
             Slug for this line?"
        );
    }
}

/// Highest luminance in the frame, or `None` for an all-black (no-ink) frame.
fn frame_max_luminance(pixels: &[u8], width: u32, height: u32) -> Option<u8> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| luminance(pixels, width, x, y))
        .max()
        .filter(|&m| m > 0)
}

/// The `[min_y, max_y)` row range containing any "ink" pixel (R > half the
/// frame's max luminance), or `None` for a blank frame. Used to band a
/// render by its ACTUAL text extent rather than the padded viewport.
fn ink_row_bounds(pixels: &[u8], width: u32, height: u32) -> Option<(u32, u32)> {
    let half = f32::from(frame_max_luminance(pixels, width, height)?) * 0.5;
    let mut min_y = None;
    let mut max_y = 0u32;
    for y in 0..height {
        let has_ink = (0..width).any(|x| f32::from(luminance(pixels, width, x, y)) > half);
        if has_ink {
            min_y.get_or_insert(y);
            max_y = y + 1;
        }
    }
    min_y.map(|y0| (y0, max_y))
}

/// Fraction of "ink" pixels (R > half of the frame's max luminance) within
/// rows `[y0, y1)` — used to check ink is spread across the render, not
/// just present somewhere.
fn ink_fraction_band(pixels: &[u8], width: u32, height: u32, y0: u32, y1: u32) -> f32 {
    let y1 = y1.min(height);
    if y1 <= y0 {
        return 0.0;
    }
    let Some(max_lum) = frame_max_luminance(pixels, width, height) else {
        return 0.0;
    };
    let half = f32::from(max_lum) * 0.5;
    let total = (width * (y1 - y0)) as f32;
    let count = (y0..y1)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .filter(|&(x, y)| f32::from(luminance(pixels, width, x, y)) > half)
        .count();
    count as f32 / total
}

/// Fraction of bytes in `a`/`b` (same length assumed — callers assert
/// matching dimensions first) that differ by more than a small tolerance —
/// tolerating harmless sRGB/blend rounding while catching a genuinely
/// different render (wrong atlas texels, misplaced quads).
const PIXEL_DIFF_TOLERANCE: u8 = 4;

fn pixel_diff_fraction(a: &[u8], b: &[u8]) -> f32 {
    if a.is_empty() {
        return 0.0;
    }
    let mismatched = a
        .iter()
        .zip(b.iter())
        .filter(|&(&x, &y)| x.abs_diff(y) > PIXEL_DIFF_TOLERANCE)
        .count();
    mismatched as f32 / a.len() as f32
}

fn luminance(pixels: &[u8], width: u32, x: u32, y: u32) -> u8 {
    pixels[((y * width + x) * BYTES_PER_PIXEL) as usize]
}

/// Render `content` through the real embed path, white-on-black.
fn render(renderer: &mut WgpuRenderer, content: &str) -> (Vec<u8>, u32, u32) {
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();
    let phys_w = LOGICAL_W.round() as u32;
    let phys_h = LOGICAL_H.round() as u32;
    let texture = create_capture_texture(&device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let scene = build_text_scene(content);
    renderer
        .submit_screen(
            &ScreenSubmission {
                device: &device,
                queue: &queue,
                view: &view,
                format: FORMAT,
                viewport: (LOGICAL_W, LOGICAL_H),
                scale: 1.0,
                clear: Some(sigil_hal::Color::BLACK),
            },
            &scene,
        )
        .expect("submit_screen");

    let pixels = read_pixels(&device, &queue, &texture, phys_w, phys_h);
    (pixels, phys_w, phys_h)
}

fn build_text_scene(content: &str) -> CommandScene {
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: FONT_SIZE,
        font_weight: 400,
        line_height: TextLineHeight::FontSizeRelative(1.2),
    };
    let rect = sigil_hal::Rect::new(
        sigil_hal::Point::new(ORIGIN.0, ORIGIN.1),
        LOGICAL_W - ORIGIN.0,
        LOGICAL_H - ORIGIN.1,
    );
    let run = shape_glyph_run(
        rect,
        content.to_string(),
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

// ---- wgpu plumbing --------------------------------------------------------

fn try_headless_renderer() -> Option<WgpuRenderer> {
    let mut renderer = WgpuRenderer::new().ok()?;
    renderer.ensure_headless_for_tests().ok()?;
    Some(renderer)
}

fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("raster-atlas-growth capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Copy `texture` into a fresh mapped-readback buffer and block until the
/// GPU copy completes. Split out of [`read_pixels`] to stay under the
/// 50-line function cap.
fn copy_to_readback_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: (u32, u32),
    padded_bpr: u32,
) -> wgpu::Buffer {
    let (width, height) = size;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("raster-atlas-growth readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("raster-atlas-growth copy encoder"),
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bpr),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(core::iter::once(enc.finish()));

    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll");
    rx.recv().expect("map callback").expect("map result");
    buffer
}

fn read_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let unpadded_bpr = width * BYTES_PER_PIXEL;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bpr = unpadded_bpr.div_ceil(align) * align;
    let buffer = copy_to_readback_buffer(device, queue, texture, (width, height), padded_bpr);

    let slice = buffer.slice(..);
    let mapped = slice.get_mapped_range();
    let un = unpadded_bpr as usize;
    let pad = padded_bpr as usize;
    let h = height as usize;
    let mut out = vec![0u8; un * h];
    for row in 0..h {
        out[row * un..row * un + un].copy_from_slice(&mapped[row * pad..row * pad + un]);
    }
    drop(mapped);
    buffer.unmap();
    out
}
