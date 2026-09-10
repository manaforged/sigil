#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use std::sync::mpsc;

use pollster::FutureExt;
use sigil_renderer::{
    CommandScene, DrawParams, SceneBuilder, ScreenSubmission, SlugTextPipeline, WgpuRenderer,
    shape_glyph_run,
};
use sigil_style::{Rgba8, TextLineHeight, TextStyle};
use sigil_text::{OwnedTextEngine, TextConfig};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
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
    let Some((device, queue)) = try_request_device() else {
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

fn build_text_scene() -> CommandScene {
    let style = TextStyle {
        font_family: String::from("system-ui"),
        font_size: FONT_SIZE,
        font_weight: 400,
        line_height: TextLineHeight::FontSizeRelative(1.2),
        letter_spacing_em: 0.0,
        ..Default::default()
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

const POLARITY_TOLERANCE: f32 = 0.15;

const POLARITY_TEXT: &str = "sample";

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn polarity_stroke_weight_parity_scale_1_0() {
    assert_polarity_parity(1.0);
}
#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn polarity_stroke_weight_parity_scale_2_0() {
    assert_polarity_parity(2.0);
}

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

fn rel_diff(a: f32, b: f32) -> f32 {
    let m = a.max(b);
    if m <= f32::EPSILON {
        0.0
    } else {
        (a - b).abs() / m
    }
}

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
        letter_spacing_em: 0.0,
        ..Default::default()
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

struct Measurement {
    max_lum: u8,
    ink_fraction: f32,
    ink_mass: f32,
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

fn try_request_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
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

    let mapped = slice.get_mapped_range().expect("mapped range");
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
