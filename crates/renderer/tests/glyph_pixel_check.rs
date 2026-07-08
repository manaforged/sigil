//! Glyph pixel-sharpness check — Wave 0 (`docs/plans/2026-07-07-text-quality-and-gap-closure.md`).
//! Rasterizes real glyphs via [`renderer::SlugTextPipeline`], reads back raw
//! RGBA8 pixels, and measures the AA ramp width across a vertical stem in
//! PHYSICAL pixels. **Active**: the direct [`renderer::DrawParams`] path,
//! whose `scale` field sizes the ramp per DPI — must pass today. **Red**:
//! simulates the screen/embed path (`ScreenSubmission`/`submit_screen`),
//! which has no scale field — `#[ignore]`d until Wave 1 lands.
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
use icons as _;
use image as _;
use kernel as _;
use layout as _;
use proptest as _;
use raw_window_handle as _;
use shell as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use std::sync::mpsc;

use pollster::FutureExt as _;
use renderer::{DrawParams, SlugTextPipeline};
use style::{TextLineHeight, TextStyle};
use text::{OwnedTextEngine, TextConfig};

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

// ---- red-harness tests: embed path has no scale field (Wave 1 bug) --------

// 6x, not the more obvious 2x: shipped tuning (`sharpen: 2.0` — see
// slug/tuning.rs) narrows the AA ramp enough that a realistic 2x embed
// mismatch only softens to ~1.0px (vs ~0.0-0.5px direct), under
// RAMP_THRESHOLD_PX. The defect scales linearly with the mismatch instead of
// staying flat; 6x reliably crosses the threshold. See dispatch return for
// the 2x/8x measured values.
#[test]
#[ignore = "red until Wave 1: embed path has no DPI scale (ScreenSubmission)"]
fn embed_path_glyph_edges_are_soft_at_6x_dpi_mismatch() {
    assert_embed_path_is_sharp(6.0);
}

/// Simulates today's screen/embed path: paints a LOGICAL scene into a
/// PHYSICAL (`texture_scale`×) texture with `DrawParams.scale` stuck at 1.0
/// (no DPI field on `ScreenSubmission`) — the AA ramp is sized for 1x and
/// comes out proportionally wider once stretched onto the target.
fn assert_embed_path_is_sharp(texture_scale: f32) {
    let Some((device, queue)) = try_request_device() else {
        println!("no adapter — skipping embed_path glyph_pixel_check");
        return;
    };
    let (pixels, w, h) = render_text(&device, &queue, texture_scale, 1.0);
    assert_sharp(
        &format!("texture_scale {texture_scale} (draw_scale 1.0)"),
        &pixels,
        w,
        h,
    );
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
         (ink_fraction={:.4}). Direct path should hold this at every DPI; the embed path \
         is the known Wave 1 defect.",
        measurement.ink_fraction,
    );
}

// ---- render -----------------------------------------------------------------

/// Render `TEXT` into an offscreen RGBA texture, read back raw pixels.
/// `texture_scale` sets physical resolution; `draw_scale` is
/// `DrawParams.scale`. Equal args = correct direct path; `draw_scale=1.0`
/// with `texture_scale>1.0` = embed-path defect.
fn render_text(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_scale: f32,
    draw_scale: f32,
) -> (Vec<u8>, u32, u32) {
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
        .layout_paragraph_styled(TEXT, None, &style)
        .expect("layout_paragraph_styled");

    let mut pipeline = SlugTextPipeline::new(device, FORMAT);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("glyph-pixel-check encoder"),
    });
    clear_black(&mut encoder, &view);
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
                color: Some(hal::Color::WHITE),
                clip: None,
            },
        )
        .expect("record_paragraph");
    queue.submit(core::iter::once(encoder.finish()));

    let pixels = read_pixels(device, queue, &texture, phys_w, phys_h);
    (pixels, phys_w, phys_h)
}

fn clear_black(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("glyph-pixel-check clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
    let softest_ramp_px = (max_f >= 1.0)
        .then(|| softest_edge_ramp(pixels, width, height, max_f))
        .flatten();
    Measurement {
        max_lum,
        ink_fraction,
        softest_ramp_px,
    }
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
