//! Glyph pixel-sharpness check — Wave 0/1 (`docs/plans/2026-07-07-text-quality-and-gap-closure.md`).
//! Rasterizes real glyphs, reads back raw RGBA8 pixels, and measures the AA
//! ramp width across a vertical stem in PHYSICAL pixels. Two families:
//! - **Direct path** — [`sigil_renderer::DrawParams`] with an explicit `scale`; sharp
//!   at every DPI.
//! - **Embed path** — the real screen path (`sigil_renderer::shape_glyph_run` →
//!   `ScreenSubmission`/`submit_screen`). Wave 1 added `ScreenSubmission.scale`;
//!   these prove it is load-bearing: a matching `scale` into a 6× physical
//!   texture is sharp, `scale: 1.0` into the same texture is soft (the shipped
//!   blur). No `#[ignore]` — both must pass now.
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use proptest as _;
use raw_window_handle as _;
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use std::sync::mpsc;

use pollster::FutureExt as _;
use sigil_renderer::{
    CommandScene, DrawParams, SceneBuilder, ScreenSubmission, SlugTextPipeline, TextTuning,
    WgpuRenderer, set_text_tuning, shape_glyph_run,
};
use sigil_style::{Rgba8, TextLineHeight, TextStyle};
use sigil_text::{OwnedTextEngine, TextConfig};
use zeno as _;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

// Logical viewport + glyph placement.
const LOGICAL_W: f32 = 240.0;
const LOGICAL_H: f32 = 100.0;
const FONT_SIZE: f32 = 32.0;
const ORIGIN: (f32, f32) = (20.0, 20.0);
// "IIIIII": plain vertical bars, no crossbar — a crossbar (e.g. "H") creates
// a T-junction where analytic AA legitimately blends both axes and produced
// false-positive "wide ramp" readings unrelated to the DPI-scale defect.
const TEXT: &str = "IIIIII";

// AA ramp must be at most this many PHYSICAL pixels wide (doc target: ~1px).
const RAMP_THRESHOLD_PX: f32 = 1.75;

// ---- active tests: direct DrawParams path (scale field respected) ----------

#[test]
fn glyph_edges_sharp_at_scale_1_0() {
    assert_direct_path_is_sharp(1.0);
}
#[test]
fn glyph_edges_sharp_at_scale_1_25() {
    assert_direct_path_is_sharp(1.25);
}
#[test]
fn glyph_edges_sharp_at_scale_1_5() {
    assert_direct_path_is_sharp(1.5);
}
#[test]
fn glyph_edges_sharp_at_scale_2_0() {
    assert_direct_path_is_sharp(2.0);
}

fn assert_direct_path_is_sharp(scale: f32) {
    let Some((device, queue)) = try_request_device() else {
        println!("no adapter — skipping glyph_pixel_check at scale {scale}");
        return;
    };
    let (pixels, w, h) = render_text(&device, &queue, scale, scale);
    assert_sharp(&format!("scale {scale}"), &pixels, w, h);
}

// ---- embed-path tests: ScreenSubmission.scale is load-bearing (Wave 1) ----

// 6x, not the more obvious 2x: shipped tuning (`sharpen: 2.0` — see
// slug/tuning.rs) narrows the AA ramp enough that a realistic 2x embed
// mismatch only softens to ~1.0px (vs ~0.0-0.5px direct), under
// RAMP_THRESHOLD_PX. The defect scales linearly with the mismatch instead of
// staying flat; 6x reliably crosses the threshold. See dispatch return for
// the 2x/8x measured values.
const EMBED_TEXTURE_SCALE: f32 = 6.0;

/// Real embed/screen path: a `CommandScene` built the way an embedding host
/// (the host engine) builds one — `shape_glyph_run` + `SceneBuilder` — submitted via
/// `submit_screen` into a physically-larger-than-logical target. With
/// `ScreenSubmission.scale` set to the target's true DPI ratio, the AA ramp
/// must stay sharp — this is the Wave 1 fix.
#[test]
fn embed_path_glyph_edges_are_sharp_with_matching_scale() {
    let (pixels, w, h) = render_embed_text(EMBED_TEXTURE_SCALE);
    assert_sharp(
        &format!("embed scale {EMBED_TEXTURE_SCALE} (matching submission scale)"),
        &pixels,
        w,
        h,
    );
}

/// Companion negative case: the identical scene into the identical 6×
/// physical target, but `ScreenSubmission.scale` stuck at 1.0 (today's
/// shipped default before this field existed) — proves the field is
/// load-bearing, not just present. Must stay soft; if this regresses to
/// sharp, `scale` stopped reaching the Slug draw.
#[test]
fn embed_path_glyph_edges_are_soft_with_mismatched_scale() {
    let (pixels, w, h) = render_embed_text(1.0);
    assert_soft(
        &format!("embed scale 1.0 (mismatched — {EMBED_TEXTURE_SCALE}x physical target)"),
        &pixels,
        w,
        h,
    );
}

/// Render `TEXT` through the real screen/embed path: build a `CommandScene`
/// via `shape_glyph_run`, submit via `submit_screen` with
/// `ScreenSubmission.scale = submission_scale` into a physical texture fixed
/// at `EMBED_TEXTURE_SCALE`× the logical viewport, read back raw pixels.
fn render_embed_text(submission_scale: f32) -> (Vec<u8>, u32, u32) {
    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    renderer
        .ensure_headless_for_tests()
        .expect("headless renderer device — no adapter available for glyph_pixel_check");
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
                clear: Some(sigil_hal::Color::BLACK),
            },
            &scene,
        )
        .expect("submit_screen");

    let pixels = read_pixels(&device, &queue, &texture, phys_w, phys_h);
    (pixels, phys_w, phys_h)
}

/// Build the embed-path `CommandScene`: one glyph run positioned at `ORIGIN`,
/// shaped the same way an embedding host emits text (`shape_glyph_run`).
fn build_text_scene() -> CommandScene {
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

/// Shared assertion body for both scenario families: measure `pixels`, print
/// the evidence line, and assert the softest edge is under threshold.
fn assert_sharp(label: &str, pixels: &[u8], w: u32, h: u32) {
    let measurement = measure(pixels, w, h);
    assert!(
        measurement.ink_fraction > 0.0,
        "{label}: ink_fraction was 0 — no glyph pixels rendered ({w}x{h})"
    );
    let ramp_px = measurement.softest_ramp_px.unwrap_or_else(|| {
        panic!(
            "{label}: no ink stem found; max_luminance={}",
            measurement.max_lum
        )
    });
    println!(
        "{label}: measured ramp_px={ramp_px:.3} (ink_fraction={:.4})",
        measurement.ink_fraction
    );
    assert!(
        ramp_px <= RAMP_THRESHOLD_PX,
        "{label}: AA ramp width {ramp_px:.3}px exceeds {RAMP_THRESHOLD_PX}px threshold \
         (ink_fraction={:.4}). Direct path should hold this at every DPI; a correctly \
         DPI-scaled embed path should too.",
        measurement.ink_fraction,
    );
}

/// Inverse of [`assert_sharp`]: proves a mismatched `ScreenSubmission.scale`
/// actually degrades sharpness — i.e. the field is load-bearing, not inert.
fn assert_soft(label: &str, pixels: &[u8], w: u32, h: u32) {
    let measurement = measure(pixels, w, h);
    assert!(
        measurement.ink_fraction > 0.0,
        "{label}: ink_fraction was 0 — no glyph pixels rendered ({w}x{h})"
    );
    let ramp_px = measurement.softest_ramp_px.unwrap_or_else(|| {
        panic!(
            "{label}: no ink stem found; max_luminance={}",
            measurement.max_lum
        )
    });
    println!(
        "{label}: measured ramp_px={ramp_px:.3} (ink_fraction={:.4})",
        measurement.ink_fraction
    );
    assert!(
        ramp_px > RAMP_THRESHOLD_PX,
        "{label}: AA ramp width {ramp_px:.3}px is within {RAMP_THRESHOLD_PX}px threshold \
         (ink_fraction={:.4}) — expected a mismatched scale to soften the ramp. If this now \
         passes as sharp, ScreenSubmission.scale stopped reaching the Slug draw.",
        measurement.ink_fraction,
    );
}

// ---- polarity parity: stroke weight must match across contrast polarity ----

// Max relative difference permitted between the two contrast polarities'
// measured stroke weight. The Wave-2 per-polarity gamma defaults
// (`slug/tuning.rs`) are tuned until both land inside this — the honest
// stroke-weight-parity gate. Do NOT widen past 0.15.
const POLARITY_TOLERANCE: f32 = 0.15;

// Curvy mixed-case string for the parity check: unlike the "IIIIII" stem probe
// (whose pure vertical edges the dilation+sharpen path hard-clips, leaving no
// partial coverage for gamma to act on), curved/diagonal glyphs carry genuine
// AA-edge coverage — the exact region the per-polarity gamma transfer corrects.
const POLARITY_TEXT: &str = "sample";

#[test]
fn polarity_stroke_weight_parity_scale_1_0() {
    assert_polarity_parity(1.0);
}
#[test]
fn polarity_stroke_weight_parity_scale_2_0() {
    assert_polarity_parity(2.0);
}

/// Render [`POLARITY_TEXT`] as WHITE-on-black (light-on-dark) and BLACK-on-white
/// (dark-on-light) at `scale`, then assert the two measured stroke weights match
/// within [`POLARITY_TOLERANCE`]. Proves the per-polarity gamma transfer removes
/// the polarity-dependent stem-weight drift a single global gamma leaves behind
/// (uncorrected drift is ~19% at scale 1.0 — see the dispatch return).
///
/// Two weight metrics, both gated at 15%: `ink_mass` (mean normalised coverage,
/// integrates the AA-edge pixels) is the primary continuous measure; the
/// brief-named `ink_fraction` (50%-threshold core count) is the coarser check.
fn assert_polarity_parity(scale: f32) {
    let Some((device, queue)) = try_request_device() else {
        println!("no adapter — skipping polarity parity at scale {scale}");
        return;
    };
    let (lod_px, lw, lh) = render_text_string(
        &device,
        &queue,
        scale,
        POLARITY_TEXT,
        (sigil_hal::Color::WHITE, wgpu::Color::BLACK),
    );
    let lod = measure(&lod_px, lw, lh);
    // Dark-on-light: invert R so the shared measure (which keys "ink" to high
    // luminance) reads the dark stems on the light field as ink.
    let (dol_raw, dw, dh) = render_text_string(
        &device,
        &queue,
        scale,
        POLARITY_TEXT,
        (sigil_hal::Color::BLACK, wgpu::Color::WHITE),
    );
    let dol_px: Vec<u8> = dol_raw.iter().map(|b| 255 - b).collect();
    let dol = measure(&dol_px, dw, dh);

    let mass_rel = rel_diff(lod.ink_mass, dol.ink_mass);
    let ink_rel = rel_diff(lod.ink_fraction, dol.ink_fraction);
    println!(
        "polarity scale {scale}: LOD mass={:.5} ink={:.5} | DOL mass={:.5} ink={:.5} | \
         mass_rel={:.3} ink_rel={:.3}",
        lod.ink_mass, lod.ink_fraction, dol.ink_mass, dol.ink_fraction, mass_rel, ink_rel,
    );

    assert!(
        mass_rel <= POLARITY_TOLERANCE,
        "scale {scale}: stroke-weight (ink_mass) parity {:.1}% exceeds {:.0}% budget — \
         LOD {:.5} vs DOL {:.5}. Re-tune gamma_dark_on_light in slug/tuning.rs.",
        mass_rel * 100.0,
        POLARITY_TOLERANCE * 100.0,
        lod.ink_mass,
        dol.ink_mass,
    );
    assert!(
        ink_rel <= POLARITY_TOLERANCE,
        "scale {scale}: stroke-weight (ink_fraction) parity {:.1}% exceeds {:.0}% budget — \
         LOD {:.5} vs DOL {:.5}.",
        ink_rel * 100.0,
        POLARITY_TOLERANCE * 100.0,
        lod.ink_fraction,
        dol.ink_fraction,
    );
}

/// Relative difference of two non-negative measurements, normalised by the
/// larger. Zero when both are (near) zero — identically sharp edges are parity.
fn rel_diff(a: f32, b: f32) -> f32 {
    let m = a.max(b);
    if m <= f32::EPSILON {
        0.0
    } else {
        (a - b).abs() / m
    }
}

// ---- Wave-3 D3: raster-vs-vector sharpness comparison (measurement gate) ---

// Small sizes named in the DR-055/plan gate. All are within
// `raster::RASTER_MAX_LOGICAL_PX` (20px) — the raster path's eligibility
// ceiling — so both families are reachable at every size below.
const RASTER_COMPARE_SIZES_PX: [f32; 3] = [12.0, 14.0, 16.0];

#[test]
fn raster_vs_vector_sharpness_at_scale_1_0() {
    measure_raster_vs_vector(1.0);
}
#[test]
fn raster_vs_vector_sharpness_at_scale_1_25() {
    measure_raster_vs_vector(1.25);
}
#[test]
fn raster_vs_vector_sharpness_at_scale_1_5() {
    measure_raster_vs_vector(1.5);
}
#[test]
fn raster_vs_vector_sharpness_at_scale_2_0() {
    measure_raster_vs_vector(2.0);
}

/// Wave-3 D3 measurement gate (DR-055 §3, plan Wave 3): for each of
/// [`RASTER_COMPARE_SIZES_PX`], render `TEXT` through the real embed path
/// (`shape_glyph_run` → `submit_screen`, the same path production hosts
/// use) twice at matched texture/submission `scale` — once with
/// `TextTuning.raster` on, once off — and print ramp_px/ink_fraction side
/// by side.
///
/// This is a DATA-COLLECTION test, not a winner-take-all gate: it asserts
/// both paths actually produce ink (a broken raster pass would silently
/// render nothing, which is a real bug this test would catch) but does
/// NOT assert raster beats vector on sharpness — the default-on decision
/// is the driver's, made from this printed evidence, per the dispatch
/// brief ("do not flip the default yourself").
fn measure_raster_vs_vector(scale: f32) {
    for &size_px in &RASTER_COMPARE_SIZES_PX {
        let (vec_px, vw, vh) = render_embed_at(scale, size_px, false);
        let vector = measure(&vec_px, vw, vh);
        let (ras_px, rw, rh) = render_embed_at(scale, size_px, true);
        let raster = measure(&ras_px, rw, rh);

        let v_ramp = vector.softest_ramp_px.unwrap_or(f32::NAN);
        let r_ramp = raster.softest_ramp_px.unwrap_or(f32::NAN);
        println!(
            "raster-vs-vector scale={scale} size={size_px}px | vector ramp_px={v_ramp:.3} \
             ink={:.4} | raster ramp_px={r_ramp:.3} ink={:.4}",
            vector.ink_fraction, raster.ink_fraction,
        );
        assert!(
            vector.ink_fraction > 0.0,
            "scale {scale} size {size_px}: vector path produced no ink"
        );
        assert!(
            raster.ink_fraction > 0.0,
            "scale {scale} size {size_px}: raster path produced no ink — raster pass broken?"
        );
    }
}

/// Render `TEXT` at `size_px` through the real embed path with texture and
/// submission scale matched (no DPI-mismatch defect in play — raster-vs-
/// vector is the only variable). Toggles `TextTuning.raster` for the
/// duration; a [`TuningGuard`] restores the default even if an assertion
/// panics, since the flag is a shared process global other tests read.
fn render_embed_at(scale: f32, size_px: f32, raster: bool) -> (Vec<u8>, u32, u32) {
    let _guard = sigil_renderer::lock_global_text_state();
    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    renderer
        .ensure_headless_for_tests()
        .expect("headless renderer device — no adapter available for glyph_pixel_check");
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();

    let phys_w = (LOGICAL_W * scale).round() as u32;
    let phys_h = (LOGICAL_H * scale).round() as u32;
    let texture = create_capture_texture(&device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    set_text_tuning(TextTuning {
        raster,
        ..TextTuning::default()
    });

    let scene = build_text_scene_sized(size_px);
    renderer
        .submit_screen(
            &ScreenSubmission {
                device: &device,
                queue: &queue,
                view: &view,
                format: FORMAT,
                viewport: (LOGICAL_W, LOGICAL_H),
                scale,
                clear: Some(sigil_hal::Color::BLACK),
            },
            &scene,
        )
        .expect("submit_screen");

    let pixels = read_pixels(&device, &queue, &texture, phys_w, phys_h);
    (pixels, phys_w, phys_h)
}

/// Like [`build_text_scene`] but with a configurable font size — the
/// raster path's eligibility gate is `font_size <= RASTER_MAX_LOGICAL_PX`
/// (20px), so the small sizes in [`RASTER_COMPARE_SIZES_PX`] are the ones
/// that matter here.
fn build_text_scene_sized(size_px: f32) -> CommandScene {
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: size_px,
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

// ---- render -----------------------------------------------------------------

/// Render `TEXT` into an offscreen RGBA texture, read back raw pixels.
/// `texture_scale` sets physical resolution; `draw_scale` is
/// `DrawParams.scale`. Equal args = correct direct path; `draw_scale=1.0`
/// with `texture_scale>1.0` = embed-path defect. White text on black.
fn render_text(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_scale: f32,
    draw_scale: f32,
) -> (Vec<u8>, u32, u32) {
    render_text_inner(
        &RenderArgs {
            texture_scale,
            draw_scale,
            content: TEXT,
            text: sigil_hal::Color::WHITE,
            background: wgpu::Color::BLACK,
        },
        device,
        queue,
    )
}

/// Render `content` at `scale` (physical == draw scale) with an explicit text
/// and background colour, for the contrast-polarity parity checks.
fn render_text_string(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scale: f32,
    content: &str,
    colors: (sigil_hal::Color, wgpu::Color),
) -> (Vec<u8>, u32, u32) {
    render_text_inner(
        &RenderArgs {
            texture_scale: scale,
            draw_scale: scale,
            content,
            text: colors.0,
            background: colors.1,
        },
        device,
        queue,
    )
}

/// Grouped render inputs — keeps the render helper under the 5-arg cap.
struct RenderArgs<'a> {
    texture_scale: f32,
    draw_scale: f32,
    content: &'a str,
    text: sigil_hal::Color,
    background: wgpu::Color,
}

fn render_text_inner(
    args: &RenderArgs<'_>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (Vec<u8>, u32, u32) {
    let &RenderArgs {
        texture_scale,
        draw_scale,
        content,
        text,
        background,
    } = args;
    let phys_w = (LOGICAL_W * texture_scale).round() as u32;
    let phys_h = (LOGICAL_H * texture_scale).round() as u32;

    let texture = create_capture_texture(device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let engine = OwnedTextEngine::new(TextConfig::default());
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: FONT_SIZE,
        font_weight: 400,
        line_height: TextLineHeight::FontSizeRelative(1.2),
    };
    let paragraph = engine
        .layout_paragraph_styled(content, None, &style)
        .expect("layout_paragraph_styled");

    let mut pipeline = SlugTextPipeline::new(device, FORMAT);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("glyph-pixel-check encoder"),
    });
    clear_to(&mut encoder, &view, background);
    pipeline
        .record_paragraph(
            &mut encoder,
            &DrawParams {
                device,
                queue,
                view: &view,
                surface_width: LOGICAL_W,
                surface_height: LOGICAL_H,
                scale: draw_scale,
                paragraph: &paragraph,
                glyphs: None,
                origin: ORIGIN,
                color: Some(text),
                clip: None,
            },
        )
        .expect("record_paragraph");
    queue.submit(core::iter::once(encoder.finish()));

    let pixels = read_pixels(device, queue, &texture, phys_w, phys_h);
    (pixels, phys_w, phys_h)
}

fn clear_to(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, color: wgpu::Color) {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("glyph-pixel-check clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

// ---- sharpness metrics --------------------------------------------------------

struct Measurement {
    max_lum: u8,
    ink_fraction: f32,
    /// Mean normalised coverage across every pixel (Σ luminance/max ÷ N). Unlike
    /// `ink_fraction`'s 50%-threshold count, this integrates the *partial*-
    /// coverage AA-edge pixels — exactly the region the per-polarity gamma
    /// transfer acts on — so it is the continuous stroke-weight-parity metric.
    ink_mass: f32,
    /// The widest AA ramp (physical px) found on any glyph edge in the
    /// image. Taking the MAX rather than a single "widest run" avoids being
    /// fooled by a run whose boundary happens to land on an exact pixel
    /// while a neighbouring edge in the same render is genuinely soft.
    softest_ramp_px: Option<f32>,
}

fn measure(pixels: &[u8], width: u32, height: u32) -> Measurement {
    let max_lum = global_max_luminance(pixels, width, height);
    let max_f = f32::from(max_lum);
    let ink_fraction = ink_fraction(pixels, width, height, max_f);
    let ink_mass = ink_mass(pixels, width, height, max_f);
    let softest_ramp_px = (max_f >= 1.0)
        .then(|| softest_edge_ramp(pixels, width, height, max_f))
        .flatten();
    Measurement {
        max_lum,
        ink_fraction,
        ink_mass,
        softest_ramp_px,
    }
}

/// Mean normalised luminance over every pixel — the total ink "mass", including
/// partial-coverage AA-edge pixels. `0.0` for an empty frame.
fn ink_mass(pixels: &[u8], width: u32, height: u32, max_lum: f32) -> f32 {
    if max_lum < 1.0 {
        return 0.0;
    }
    let total = (width * height) as f32;
    let sum: f32 = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| f32::from(luminance(pixels, width, x, y)) / max_lum)
        .sum();
    sum / total
}

// Text is achromatic; the R channel alone carries luminance.
fn luminance(pixels: &[u8], width: u32, x: u32, y: u32) -> u8 {
    pixels[((y * width + x) * BYTES_PER_PIXEL) as usize]
}
fn global_max_luminance(pixels: &[u8], width: u32, height: u32) -> u8 {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| luminance(pixels, width, x, y))
        .max()
        .unwrap_or(0)
}
fn ink_fraction(pixels: &[u8], width: u32, height: u32, max_lum: f32) -> f32 {
    let half = 0.5 * max_lum;
    let total = (width * height) as f32;
    let count = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .filter(|&(x, y)| f32::from(luminance(pixels, width, x, y)) > half)
        .count();
    count as f32 / total
}

/// Scan every row, find every solid-ink run's two edges, and return the
/// widest AA ramp found anywhere in the image.
fn softest_edge_ramp(pixels: &[u8], width: u32, height: u32, max_lum: f32) -> Option<f32> {
    let lower = 0.10 * max_lum;
    let upper = 0.90 * max_lum;
    let mut worst: Option<f32> = None;
    for y in 0..height {
        let row: Vec<u8> = (0..width).map(|x| luminance(pixels, width, x, y)).collect();
        for run in solid_runs(&row, upper) {
            for (from, step) in [(run.0, -1), (run.1.saturating_sub(1), 1)] {
                let ramp_px = count_ramp(&row, from, step, lower, upper) as f32;
                if worst.is_none_or(|w| ramp_px > w) {
                    worst = Some(ramp_px);
                }
            }
        }
    }
    worst
}

/// Every maximal run of pixels whose luminance is ≥ `upper`, as `[start, end)`.
fn solid_runs(row: &[u8], upper: f32) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < row.len() {
        if f32::from(row[i]) >= upper {
            let start = i;
            while i < row.len() && f32::from(row[i]) >= upper {
                i += 1;
            }
            runs.push((start, i));
        } else {
            i += 1;
        }
    }
    runs
}

/// Walk from `from` (a "high" pixel) in `step` direction, counting pixels
/// strictly between `lower`/`upper` AND strictly decreasing vs. the previous
/// one — a genuine AA falloff, distinguishing it from a flat plateau at the
/// same intensity band (e.g. a crossbar's own partial coverage).
fn count_ramp(row: &[u8], from: usize, step: i32, lower: f32, upper: f32) -> usize {
    let mut count = 0usize;
    let mut prev = f32::from(row[from]);
    let mut idx = from as i64 + i64::from(step);
    while idx >= 0 && (idx as usize) < row.len() {
        let v = f32::from(row[idx as usize]);
        if v > lower && v < upper && v < prev {
            count += 1;
            prev = v;
            idx += i64::from(step);
        } else {
            break;
        }
    }
    count
}

// ---- wgpu plumbing ------------------------------------------------------------

fn try_request_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .block_on()
        .ok()?;
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("glyph-pixel-check device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .ok()
}

fn create_capture_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glyph-pixel-check capture"),
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
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glyph-pixel-check readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("glyph-pixel-check copy encoder"),
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
