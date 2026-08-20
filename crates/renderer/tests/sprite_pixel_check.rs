//! Pixel-verification test for Phase R7 three-way sprite split (Mono / Subpixel / Poly).
//!
//! Renders the R7 showcase scene via the offscreen capture path, reads back
//! raw RGBA8 bytes (no PNG roundtrip), and asserts spatial + colour contracts
//! for all three sprite variants.
//!
//! This file will not compile until the implementer's R7 work lands — that is
//! expected. Once it lands, all four assertions must pass.
//!
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal as _;
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use std::sync::mpsc;

use pollster::FutureExt as _;
use sigil_renderer::{OffscreenTarget, WgpuRenderer};

// Physical scale factor: texture dims = viewport × SCALE.
// wgpu downlevel default texture-dimension cap is 2048 px.
// Viewport 900 × scale 2 = 1800 px — safe, but keep SCALE = 1
// to match the pattern_pixel_check convention and avoid any
// adapter that enforces the 2048-px cap strictly.
const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

// Sprite destination origins and size, matching scenes::sprite() contract.
const MONO_X: f32 = 80.0;
const MONO_Y: f32 = 80.0;
const SUB_X: f32 = 360.0;
const SUB_Y: f32 = 80.0;
const POLY_X: f32 = 640.0;
const POLY_Y: f32 = 80.0;
const SPRITE_SIDE: f32 = 200.0;

/// Render the R7 sprite scene and assert pixel-level contracts for all three
/// sprite variants. Skips cleanly when no GPU adapter is available.
#[test]
fn sprite_pixels_match_mono_subpixel_poly_contracts() {
    let Some((device, queue)) = try_request_device() else {
        println!("no adapter — skipping sprite_pixel_check");
        return;
    };

    let pixels = render_sprite_scene(&device, &queue);
    let phys_w = viewport_phys_w();

    assert_mono_tint_is_red(&pixels, phys_w);
    assert_mono_corner_is_not_red_tint(&pixels, phys_w);
    assert_subpixel_rgb_independence(&pixels, phys_w);
    assert_poly_tint_ignored_green_atlas(&pixels, phys_w);
}

// ---- render -----------------------------------------------------------------

fn viewport_phys_w() -> u32 {
    // scenes::sprite() returns the viewport; we replicate the width here so
    // the assertion helpers can compute byte offsets without re-calling it.
    // The plan locks the viewport to (900, 400) logical pixels.
    (900.0_f32 * SCALE as f32) as u32
}

fn render_sprite_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let (command_scene, viewport, clear) = sigil_renderer::scenes::sprite();

    let phys_w = (viewport.0 * SCALE as f32) as u32;
    let phys_h = (viewport.1 * SCALE as f32) as u32;

    let texture = create_capture_texture(device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport,
                scale: 1.0,
            },
            &command_scene,
            clear,
        )
        .expect("render_command_scene_offscreen");

    read_pixels(device, queue, &texture, phys_w, phys_h)
}

// ---- assertion 1: mono tint is red at center --------------------------------

/// Center of the Mono sprite must be RED-tinted.
/// Locked tint: Rgba8 { r: 220, g: 60, b: 60, a: 255 }.
/// After sRGB + coverage compositing, center should satisfy R > 180, G < 100, B < 100.
fn assert_mono_tint_is_red(pixels: &[u8], phys_w: u32) {
    let (cx, cy) = lp(MONO_X + SPRITE_SIDE / 2.0, MONO_Y + SPRITE_SIDE / 2.0);
    let rgba = pixel_rgba(pixels, phys_w, cx, cy);
    assert!(
        rgba[0] > 180 && rgba[1] < 100 && rgba[2] < 100,
        "Mono tint: center pixel ({cx},{cy}) should be red-tinted \
         (R > 180, G < 100, B < 100). Got R={} G={} B={}. \
         Check sprite_paint.wgsl SPRITE_KIND_MONO branch: \
         coverage * in.color.rgb should amplify R and suppress G/B.",
        rgba[0],
        rgba[1],
        rgba[2],
    );
}

// ---- assertion 2: mono corner does NOT match the tint -----------------------

/// A corner of the Mono sprite rect (5 px inside, where the soft disk alpha
/// is near zero) should composite to something close to the clear color,
/// NOT to the red tint. Checks that the disk shape actually rendered.
fn assert_mono_corner_is_not_red_tint(pixels: &[u8], phys_w: u32) {
    let (cx, cy) = lp(MONO_X + 5.0, MONO_Y + 5.0);
    let rgba = pixel_rgba(pixels, phys_w, cx, cy);

    // Red tint reference: (220, 60, 60).
    let delta_to_tint = color_distance(rgba, [220, 60, 60, 255]);
    // Clear color is dark (from scenes::sprite()); any dark-ish pixel is fine.
    // We only require that the corner is NOT approximately the tint color —
    // i.e., it's farther from the tint than from solid black.
    let delta_to_black = color_distance(rgba, [0, 0, 0, 255]);

    assert!(
        delta_to_tint > delta_to_black,
        "Mono shape: corner pixel ({cx},{cy}) looks like the tint color rather than \
         the clear color. R={} G={} B={} A={}. \
         dist_to_tint={delta_to_tint} must exceed dist_to_black={delta_to_black}. \
         The synthetic disk should have near-zero alpha at the rect corners.",
        rgba[0],
        rgba[1],
        rgba[2],
        rgba[3],
    );
}

// ---- assertion 3: subpixel RGB independence ---------------------------------

/// At the right edge of the Subpixel sprite, scan three adjacent horizontal
/// pixels and require that at least one channel at one pixel differs from
/// another channel of the same pixel by ≥ 16 units. This is intentionally
/// loose — it just proves the RGB channels were evaluated independently.
fn assert_subpixel_rgb_independence(pixels: &[u8], phys_w: u32) {
    let (ex, ey) = lp(SUB_X + SPRITE_SIDE - 5.0, SUB_Y + SPRITE_SIDE / 2.0);

    let found = (0..3_u32).any(|dx| {
        let px = pixel_rgba(pixels, phys_w, ex.saturating_sub(dx), ey);
        let r = px[0] as i32;
        let g = px[1] as i32;
        let b = px[2] as i32;
        (r - g).abs() >= 16 || (r - b).abs() >= 16 || (g - b).abs() >= 16
    });

    assert!(
        found,
        "Subpixel RGB independence: scanned three pixels around ({ex},{ey}). \
         None had any channel differing from another channel by ≥ 16. \
         Check sprite_paint.wgsl SPRITE_KIND_SUBPIXEL branch: \
         R/G/B must sample three phase-offset atlas positions independently."
    );
}

// ---- assertion 4: poly tint is ignored, atlas green wins --------------------

/// Center of the Poly sprite: the synthetic atlas has bright green at the
/// poly region center (horizontal red→green gradient, fully opaque).
/// The tint color supplied by scenes::sprite() is IGNORED for Poly.
/// Assert G > 180 && R < 120 to prove atlas content dominates.
fn assert_poly_tint_ignored_green_atlas(pixels: &[u8], phys_w: u32) {
    let (cx, cy) = lp(POLY_X + SPRITE_SIDE / 2.0, POLY_Y + SPRITE_SIDE / 2.0);
    let rgba = pixel_rgba(pixels, phys_w, cx, cy);
    assert!(
        rgba[1] > 180 && rgba[0] < 120,
        "Poly tint-ignored: center pixel ({cx},{cy}) should be bright green \
         from the atlas (G > 180, R < 120). Got R={} G={} B={}. \
         The Poly atlas region is a horizontal red→green gradient; center is \
         ~50% across, so G dominates. Check sprite_paint.wgsl SPRITE_KIND_POLY \
         branch: tint must be dropped, raw atlas RGBA passed through.",
        rgba[0],
        rgba[1],
        rgba[2],
    );
}

// ---- pixel helpers ----------------------------------------------------------

/// Logical-to-physical coordinate conversion.
fn lp(lx: f32, ly: f32) -> (u32, u32) {
    ((lx * SCALE as f32) as u32, (ly * SCALE as f32) as u32)
}

fn pixel_rgba(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let b = ((y * width + x) * BYTES_PER_PIXEL) as usize;
    [pixels[b], pixels[b + 1], pixels[b + 2], pixels[b + 3]]
}

/// Manhattan distance between two RGBA pixels (alpha included).
fn color_distance(a: [u8; 4], b: [u8; 4]) -> u32 {
    (0..4)
        .map(|i| (a[i] as i32 - b[i] as i32).unsigned_abs())
        .sum()
}

// ---- wgpu plumbing ----------------------------------------------------------

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
            label: Some("sprite-pixel-check device"),
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
        label: Some("sprite-pixel-check capture"),
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
    let padded_bpr = {
        let r = unpadded_bpr % align;
        if r == 0 {
            unpadded_bpr
        } else {
            unpadded_bpr + align - r
        }
    };
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sprite-pixel-check readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("sprite-pixel-check copy encoder"),
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
