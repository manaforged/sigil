//! Pixel-verification test for Phase R6 pattern fills (Slash + Checker).
//!
//! Renders a deterministic scene via the offscreen capture path, reads back
//! raw RGBA8 bytes (no PNG roundtrip), and asserts spatial + colour properties.
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
use hal as _;
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
use style as _;
use text as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use std::sync::mpsc;

use hal::{Color, Point, Rect};
use pollster::FutureExt as _;
use renderer::{
    CornerRadii, OffscreenTarget, PatternKind, SceneBuilder, ScenePatternStyle, WgpuRenderer,
};
use style::Rgba8;

// Physical scale factor: texture dims = viewport × SCALE.
// wgpu's downlevel default texture-dimension limit is 2048 px on each
// axis. Viewport 1200 × scale 2 = 2400 px exceeds it, so we stay at
// scale 1 — period 40 logical = 40 physical still gives 30 cycles
// across the 1200-px-wide capture, plenty of transitions to assert
// against without rebuilding the device with a custom limit.
const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;
const VIEWPORT: (f32, f32) = (1200.0, 400.0);

const SX: f32 = 80.0;
const SY: f32 = 80.0;
const SW: f32 = 480.0;
const SH: f32 = 240.0;

const CX: f32 = 620.0;
const CY: f32 = 80.0;

/// Renders the R6 pattern scene and asserts pixel-level contracts.
/// Skips cleanly when no GPU adapter is available (headless CI).
#[test]
fn pattern_pixels_match_slash_and_checker_contracts() {
    let Some((device, queue)) = try_request_device() else {
        println!("no adapter — skipping pattern_pixel_check");
        return;
    };

    let pixels = render_pattern_scene(&device, &queue);
    let phys_w = (VIEWPORT.0 * SCALE as f32) as u32;

    assert_slash_transitions(&pixels, phys_w);
    assert_slash_directionality(&pixels, phys_w);
    assert_checker_alternation(&pixels, phys_w);
    assert_checker_colour_sanity(&pixels, phys_w);
}

fn build_pattern_scene() -> renderer::CommandScene {
    let slash = PatternKind::Slash {
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        period: 40.0,
        line_width: 16.0,
    };
    let checker = PatternKind::Checker {
        color_a: Rgba8 {
            r: 220,
            g: 60,
            b: 60,
            a: 255,
        },
        color_b: Rgba8 {
            r: 245,
            g: 245,
            b: 250,
            a: 255,
        },
        cell: 32.0,
    };
    let mut scene = SceneBuilder::new();
    scene.draw_pattern(
        Rect::new(Point::new(SX, SY), SW, SH),
        ScenePatternStyle::new(slash, CornerRadii::ZERO, 10_000),
    );
    scene.draw_pattern(
        Rect::new(Point::new(CX, CY), 480.0, 240.0),
        ScenePatternStyle::new(checker, CornerRadii::ZERO, 10_000),
    );
    scene.finish()
}

fn render_pattern_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let phys_w = (VIEWPORT.0 * SCALE as f32) as u32;
    let phys_h = (VIEWPORT.1 * SCALE as f32) as u32;
    let texture = create_capture_texture(device, phys_w, phys_h);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let scene = build_pattern_scene();
    WgpuRenderer::new()
        .expect("WgpuRenderer::new")
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: VIEWPORT,
            },
            &scene,
            Color::rgba(0.06, 0.07, 0.10, 1.0),
        )
        .expect("render_command_scene_offscreen");
    read_pixels(device, queue, &texture, phys_w, phys_h)
}

// ---- assertion 1: slash transitions ------------------------------------

/// Scans a 32 physical-px horizontal band at mid-y of the slash rect.
/// Counts dark→bright R-channel transitions. Requires ≥ 4 across ~960 px.
fn assert_slash_transitions(pixels: &[u8], phys_w: u32) {
    let mid_y_phys = ((SY + SH / 2.0) * SCALE as f32) as u32;
    let band_start = mid_y_phys.saturating_sub(16);
    let band_end = (mid_y_phys + 16).min((VIEWPORT.1 * SCALE as f32) as u32 - 1);
    let x_start = (SX * SCALE as f32) as u32;
    let x_end = ((SX + SW) * SCALE as f32) as u32;

    let mut max_transitions = 0u32;
    for y in band_start..=band_end {
        let mut transitions = 0u32;
        let mut prev_bright = false;
        for x in x_start..x_end {
            let r = pixel_r(pixels, phys_w, x, y);
            if r < 80 && prev_bright {
                transitions += 1;
                prev_bright = false;
            } else if r > 200 {
                prev_bright = true;
            }
        }
        if transitions > max_transitions {
            max_transitions = transitions;
        }
    }
    assert!(
        max_transitions >= 4,
        "slash: expected ≥ 4 dark→bright transitions in horizontal scan at \
         mid-y logical; got {max_transitions}. Period=40 × scale=2 = 80 px/cycle \
         across 960 px gives ~12 cycles. Check fill_paint.wgsl fill_kind=1 branch."
    );
}

// ---- assertion 2: slash 45° directionality ------------------------------

/// Points A and B share the same `x − y` diagonal (same stripe phase
/// since the slash math hashes on `x - y`). Point C is offset only
/// horizontally from A by 20 logical px — `x - y` shifts by 20, which
/// is half a 40-px period, so C lands at the opposite stripe phase.
fn assert_slash_directionality(pixels: &[u8], phys_w: u32) {
    let (ax, ay) = lp(SX + 20.0, SY + 20.0);
    let (bx, by) = lp(SX + 60.0, SY + 60.0); // same x−y as A (diff 0)
    let (cx, cy) = lp(SX + 40.0, SY + 20.0); // x−y shifted by +20 (half period)

    let ra = pixel_r(pixels, phys_w, ax, ay) as i32;
    let rb = pixel_r(pixels, phys_w, bx, by) as i32;
    let rc = pixel_r(pixels, phys_w, cx, cy) as i32;

    assert!(
        (ra - rb).abs() < 30,
        "slash 45°: A({ax},{ay}) R={ra} and B({bx},{by}) R={rb} share the same \
         diagonal — diff {} should be < 30. Horizontal stripes would fail here.",
        (ra - rb).abs()
    );
    assert!(
        (ra - rc).abs() > 60,
        "slash 45°: C({cx},{cy}) R={rc} is half-period offset from A R={ra} \
         (opposite stripe phase) — diff {} must be > 60. \
         Constant solid/blank render or wrong slope would fail here.",
        (ra - rc).abs()
    );
}

// ---- assertion 3: checker 2×2 alternation --------------------------------

/// Four cell-centre samples in a 2×2 grid must alternate reddish/whitish.
fn assert_checker_alternation(pixels: &[u8], phys_w: u32) {
    let [a, b, c, d] = four_checker_samples(pixels, phys_w);
    let pat = [is_reddish(a), is_reddish(b), is_reddish(c), is_reddish(d)];
    assert!(
        pat == [true, false, false, true] || pat == [false, true, true, false],
        "checker 2×2: expected alternating reddish/whitish; got {pat:?}. \
         A={a:?} B={b:?} C={c:?} D={d:?}."
    );
}

// ---- assertion 4: checker colour sanity ---------------------------------

/// At least one reddish cell matching color_a and one whitish matching color_b.
fn assert_checker_colour_sanity(pixels: &[u8], phys_w: u32) {
    let samples = four_checker_samples(pixels, phys_w);
    assert!(
        samples.iter().any(|&s| is_reddish_tight(s)),
        "checker: no reddish cell matching color_a rgba(220,60,60) within \
         sRGB bounds R∈[200,240], G∈[40,80], B∈[40,80]. Samples: {samples:?}"
    );
    assert!(
        samples.iter().any(|&s| is_whitish(s)),
        "checker: no whitish cell matching color_b rgba(245,245,250) \
         (R,G,B all > 200). Samples: {samples:?}"
    );
}

// ---- helpers -------------------------------------------------------------

/// Logical-to-physical coordinate conversion.
fn lp(lx: f32, ly: f32) -> (u32, u32) {
    ((lx * SCALE as f32) as u32, (ly * SCALE as f32) as u32)
}

/// Returns [A, B, C, D] RGBA samples centred in 2×2 adjacent checker cells.
fn four_checker_samples(pixels: &[u8], phys_w: u32) -> [[u8; 4]; 4] {
    let (ax, ay) = lp(CX + 16.0, CY + 16.0);
    let (bx, by) = lp(CX + 48.0, CY + 16.0);
    let (cx, cy) = lp(CX + 16.0, CY + 48.0);
    let (dx, dy) = lp(CX + 48.0, CY + 48.0);
    [
        pixel_rgba(pixels, phys_w, ax, ay),
        pixel_rgba(pixels, phys_w, bx, by),
        pixel_rgba(pixels, phys_w, cx, cy),
        pixel_rgba(pixels, phys_w, dx, dy),
    ]
}

fn pixel_r(pixels: &[u8], width: u32, x: u32, y: u32) -> u8 {
    pixel_rgba(pixels, width, x, y)[0]
}

fn pixel_rgba(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let b = ((y * width + x) * BYTES_PER_PIXEL) as usize;
    [pixels[b], pixels[b + 1], pixels[b + 2], pixels[b + 3]]
}

/// Loose: R > 180 && G < 120 (catches reddish vs whitish for alternation).
const fn is_reddish(rgba: [u8; 4]) -> bool {
    rgba[0] > 180 && rgba[1] < 120
}

/// Tight: matches color_a rgba(220,60,60) within sRGB gamma rounding.
const fn is_reddish_tight(rgba: [u8; 4]) -> bool {
    rgba[0] >= 200
        && rgba[0] <= 240
        && rgba[1] >= 40
        && rgba[1] <= 80
        && rgba[2] >= 40
        && rgba[2] <= 80
}

/// Matches color_b rgba(245,245,250): all channels > 200.
const fn is_whitish(rgba: [u8; 4]) -> bool {
    rgba[0] > 200 && rgba[1] > 200 && rgba[2] > 200
}

// ---- wgpu plumbing -------------------------------------------------------

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
            label: Some("pattern-pixel-check device"),
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
        label: Some("pattern-pixel-check capture"),
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
        label: Some("pattern-pixel-check readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("pattern-pixel-check copy encoder"),
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
