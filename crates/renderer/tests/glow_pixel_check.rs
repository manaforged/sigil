//! Pixel-verification test for Phase R10 soft glow / outer bloom.
//!
//! Renders the R10 glow showcase via the offscreen capture path, reads back raw
//! RGBA8 bytes (no PNG roundtrip), and asserts four pixel-level contracts:
//!
//! 1. No halo on the control surface (surface 0, no glow) — background pixels
//!    to its left match the clear colour within ±6 per channel.
//! 2. Cyan halo on surface 1 — B channel clearly elevated above background.
//! 3. Magenta halo on surface 2 — R and B channels clearly elevated above background.
//! 4. Halo strength is monotone: sum(surface 2 ring) > sum(surface 1 ring) > sum(surface 0 ring).
//!
//! This file will not compile until the implementer's R10 work lands
//! (`scenes::glow()`, `Shadow::glow`, `ShadowKind::Glow`). That is expected.
//! Once it lands, all four assertions must pass or the pixel-test contract
//! defined in phase-R10-soft-glow.md is violated.
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap, clippy::too_many_arguments, dead_code)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]
#![allow(clippy::many_single_char_names, clippy::similar_names)]

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
use zeno as _;

// Physical scale factor: texture dims = viewport × SCALE.
// Viewport (960, 360) × scale 1 = (960, 360) px — under the 2048-px downlevel cap.
const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

// Glow scene viewport, matching scenes::glow().
const VIEWPORT: (f32, f32) = (960.0, 360.0);

// Clear colour `Color::rgba(0.05, 0.06, 0.09, 1.0)` in Rgba8UnormSrgb bytes.
// wgpu converts linear→sRGB on write:
//   linear 0.05 → sRGB ≈ 0.2492 → byte 63
//   linear 0.06 → sRGB ≈ 0.2673 → byte 68
//   linear 0.09 → sRGB ≈ 0.3254 → byte 83
// Tolerance ±12 to absorb GPU rounding / driver variance.
const CLEAR_R: u8 = 63;
const CLEAR_G: u8 = 68;
const CLEAR_B: u8 = 83;
const CLEAR_TOL: u8 = 12;

// Halo ring sample distance outside each surface, in logical px.
const RING_DIST: f32 = 16.0;
// Corner offset (inside the diagonal) for the 4 corner ring samples.
const CORNER_DIST: f32 = 12.0;

// Surface rects (x, y, w, h) as defined by scenes::glow().
const SURF0: (f32, f32, f32, f32) = (100.0, 80.0, 200.0, 200.0);
const SURF1: (f32, f32, f32, f32) = (380.0, 80.0, 200.0, 200.0);
const SURF2: (f32, f32, f32, f32) = (660.0, 80.0, 200.0, 200.0);

// ---- entry point ------------------------------------------------------------

/// Render the R10 glow scene and assert all four pixel-level contracts.
/// Skips cleanly when no GPU adapter is available (headless CI).
#[test]
fn glow_pixel_check() {
    let Some((device, queue)) = try_request_device() else {
        println!("no adapter — skipping glow_pixel_check");
        return;
    };

    let pixels = render_glow_scene(&device, &queue);
    let phys_w = (VIEWPORT.0 * SCALE as f32) as u32;

    assert_no_halo_on_control_surface(&pixels, phys_w);
    assert_cyan_halo_on_surface_1(&pixels, phys_w);
    assert_magenta_halo_on_surface_2(&pixels, phys_w);
    assert_halo_strength_monotone(&pixels, phys_w);
}

// ---- render -----------------------------------------------------------------

fn render_glow_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    // scenes::glow() is provided by the implementer; locked API returns
    // (CommandScene, (f32, f32) viewport, Color clear).
    let (command_scene, viewport, clear) = sigil_renderer::scenes::glow();

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

// ---- assertion 1: no halo on control surface --------------------------------

/// At 20 px to the left of surface 0's left edge, the pixel must be within
/// ±CLEAR_TOL per channel of the clear colour.
///
/// Surface 0 has no glow. Its left edge is at logical x=100; the sample point
/// is at logical (80, 180). No glow contribution should shift this pixel.
fn assert_no_halo_on_control_surface(pixels: &[u8], phys_w: u32) {
    // Assertion 1 sample: 20 px left of surface 0 left edge, mid-height.
    let x = (SURF0.0 - 20.0) as u32; // logical 80
    let y = (SURF0.1 + SURF0.3 / 2.0) as u32; // logical 180

    let [r, g, b, _] = pixel_rgba(pixels, phys_w, x, y);

    assert!(
        abs_diff(r, CLEAR_R) <= CLEAR_TOL
            && abs_diff(g, CLEAR_G) <= CLEAR_TOL
            && abs_diff(b, CLEAR_B) <= CLEAR_TOL,
        "control surface: expected pixel at ({x},{y}) to match clear colour \
         ({CLEAR_R},{CLEAR_G},{CLEAR_B}) within ±{CLEAR_TOL}. \
         Got ({r},{g},{b}). Surface 0 has no glow — no halo must bleed to the left. \
         Check that ShadowKind::Glow is only emitted for surfaces that request it, \
         and that the control surface draws zero shadow quads.",
    );
}

// ---- assertion 2: cyan halo on surface 1 ------------------------------------

/// At 14 px to the left of surface 1's left edge, channel B must be clearly
/// elevated above the background, indicating a cyan glow halo.
///
/// Glow: Shadow::glow(rgba(80,200,255,128), spread=8, blur=18).
/// The B channel dominates cyan. Even at half alpha (128) the halo must be
/// visible at this distance inside the blur radius (18 logical px).
fn assert_cyan_halo_on_surface_1(pixels: &[u8], phys_w: u32) {
    // 14 px left of surface 1 left edge (380 - 14 = 366), mid-height.
    let x = (SURF1.0 - 14.0) as u32; // logical 366
    let y = (SURF1.1 + SURF1.3 / 2.0) as u32; // logical 180

    let [_r, g, b, _] = pixel_rgba(pixels, phys_w, x, y);

    let min_b: u8 = 60;
    let halo_lift: u8 = 25; // B must be at least clear_b + 25

    assert!(
        b > min_b,
        "cyan halo: B channel at ({x},{y}) = {b}; expected > {min_b}. \
         Surface 1 has Shadow::glow(rgba(80,200,255,128), spread=8, blur=18). \
         At 14 px from the surface edge (inside the 18-px blur radius) \
         the B channel must be lifted by the cyan halo. \
         Check that ShadowKind::Glow uses the same coverage math as Outer \
         and that shadow_kind=2.0 reaches the glow branch in the WGSL shader.",
    );

    let expected_b_floor = CLEAR_B.saturating_add(halo_lift);
    assert!(
        b > expected_b_floor,
        "cyan halo: B={b} at ({x},{y}) must exceed clear_b({CLEAR_B})+{halo_lift}={expected_b_floor}. \
         The cyan halo should be clearly brighter than the background in the B channel. \
         Verify that the glow color rgba(80,200,255,128) is being packed correctly \
         into the shadow instance buffer and reaches the shader.",
    );

    // G is also lifted by the cyan glow (rgba.g = 200); R should stay near clear.
    assert!(
        g > CLEAR_G,
        "cyan halo: G channel at ({x},{y}) = {g}; expected > clear_g ({CLEAR_G}). \
         The glow color has G=200, so G should also be lifted above background. \
         R should be near clear, B and G should be elevated.",
    );
}

// ---- assertion 3: magenta halo on surface 2 ---------------------------------

/// At 18 px to the left of surface 2's left edge, channels R and B must be
/// clearly elevated, indicating a magenta (R+B) glow halo.
///
/// Glow: Shadow::glow(rgba(220,80,220,180), spread=12, blur=28).
/// Magenta = red + blue. R > 80, B > 60. Both must exceed the clear baseline
/// by R > 25, B > 30.
fn assert_magenta_halo_on_surface_2(pixels: &[u8], phys_w: u32) {
    // 18 px left of surface 2 left edge (660 - 18 = 642), mid-height.
    let x = (SURF2.0 - 18.0) as u32; // logical 642
    let y = (SURF2.1 + SURF2.3 / 2.0) as u32; // logical 180

    let [r, _g, b, _] = pixel_rgba(pixels, phys_w, x, y);

    let min_r: u8 = 80;
    let min_b: u8 = 60;
    let r_lift: u8 = 25;
    let b_lift: u8 = 30;

    assert!(
        r > min_r,
        "magenta halo: R channel at ({x},{y}) = {r}; expected > {min_r}. \
         Surface 2 has Shadow::glow(rgba(220,80,220,180), spread=12, blur=28). \
         Magenta = R + B. At 18 px from edge (inside the 28-px blur) \
         R must be clearly elevated. Check ShadowKind::Glow path in shadow_shader.wgsl.",
    );

    assert!(
        b > min_b,
        "magenta halo: B channel at ({x},{y}) = {b}; expected > {min_b}. \
         Magenta glow rgba(220,80,220,180) has equal R and B contribution. \
         Both channels should be elevated above the clear baseline.",
    );

    let expected_r_floor = CLEAR_R.saturating_add(r_lift);
    assert!(
        r > expected_r_floor,
        "magenta halo: R={r} at ({x},{y}) must exceed clear_r({CLEAR_R})+{r_lift}={expected_r_floor}. \
         The magenta glow should produce measurable R lift above the dark background.",
    );

    let expected_b_floor = CLEAR_B.saturating_add(b_lift);
    assert!(
        b > expected_b_floor,
        "magenta halo: B={b} at ({x},{y}) must exceed clear_b({CLEAR_B})+{b_lift}={expected_b_floor}. \
         The magenta glow should produce measurable B lift above the dark background. \
         If B is not lifted, the shader may not be blending the glow color correctly.",
    );
}

// ---- assertion 4: halo strength is monotone ---------------------------------

/// Sample 8 points on a halo ring 16 px outside each surface.
/// Sum (R+G+B) over all 8 samples for each surface.
/// Assert s2 > s1 > s0 AND s2 - s0 > 200.
///
/// The control (s0) should be near 3 × clear_total × 8 = background.
/// The subtle cyan glow (s1) is medium. The strong magenta glow (s2) is highest.
fn assert_halo_strength_monotone(pixels: &[u8], phys_w: u32) {
    let s0 = ring_brightness_sum(pixels, phys_w, SURF0);
    let s1 = ring_brightness_sum(pixels, phys_w, SURF1);
    let s2 = ring_brightness_sum(pixels, phys_w, SURF2);

    assert!(
        s2 > s1,
        "halo monotone: expected strong glow (s2={s2}) > subtle glow (s1={s1}). \
         Surface 2 has spread=12, blur=28, alpha=180; surface 1 has spread=8, blur=18, alpha=128. \
         The stronger glow parameters must produce a brighter aggregate halo ring. \
         Check that spread and blur are carried through to the shadow instance \
         and that the shader computes wider coverage for larger sigma.",
    );

    assert!(
        s1 > s0,
        "halo monotone: expected subtle glow (s1={s1}) > control (s0={s0}). \
         Surface 1 has a cyan glow; surface 0 has none. \
         The ring samples around surface 1 must be brighter than the plain background. \
         If s1 ≤ s0 the glow is not rendering or the ring samples are outside its reach.",
    );

    let spread = s2.saturating_sub(s0);
    assert!(
        spread > 200,
        "halo monotone: s2({s2}) - s0({s0}) = {spread}; expected > 200. \
         The strong magenta glow (spread=12, blur=28) must produce substantially \
         higher aggregate brightness than the no-glow control across 8 ring samples. \
         A spread of ≤ 200 means either the glow is very dim, the sample points are \
         too far from the halo, or the shader is not blending the glow color.",
    );
}

// ---- ring sampler -----------------------------------------------------------

/// Sum (R+G+B) at 8 points on a halo ring RING_DIST px outside the surface.
fn ring_brightness_sum(pixels: &[u8], phys_w: u32, surf: (f32, f32, f32, f32)) -> u32 {
    let (x, y, w, h) = surf;
    let points = halo_ring_points(x, y, w, h);
    points
        .iter()
        .map(|&(px, py)| {
            let [r, g, b, _] = pixel_rgba(pixels, phys_w, px, py);
            u32::from(r) + u32::from(g) + u32::from(b)
        })
        .sum()
}

/// Return 8 logical sample points arranged around the halo ring outside `surf`.
fn halo_ring_points(x: f32, y: f32, w: f32, h: f32) -> [(u32, u32); 8] {
    let lp = |lx: f32, ly: f32| -> (u32, u32) { (lx as u32, ly as u32) };
    [
        lp(x - RING_DIST, y + h / 2.0),               // left mid
        lp(x + w + RING_DIST, y + h / 2.0),           // right mid
        lp(x + w / 2.0, y - RING_DIST),               // top mid
        lp(x + w / 2.0, y + h + RING_DIST),           // bottom mid
        lp(x - CORNER_DIST, y - CORNER_DIST),         // top-left corner
        lp(x + w + CORNER_DIST, y - CORNER_DIST),     // top-right corner
        lp(x - CORNER_DIST, y + h + CORNER_DIST),     // bottom-left corner
        lp(x + w + CORNER_DIST, y + h + CORNER_DIST), // bottom-right corner
    ]
}

// ---- pixel helpers ----------------------------------------------------------

fn pixel_rgba(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let base = ((y * width + x) * BYTES_PER_PIXEL) as usize;
    [
        pixels[base],
        pixels[base + 1],
        pixels[base + 2],
        pixels[base + 3],
    ]
}

fn abs_diff(a: u8, b: u8) -> u8 {
    a.abs_diff(b)
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
            label: Some("glow-pixel-check device"),
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
        label: Some("glow-pixel-check capture"),
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
        label: Some("glow-pixel-check readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("glow-pixel-check copy encoder"),
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
