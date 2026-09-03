#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Pixel-verification test for Phase R11 offscreen layer pool.
//!
//! Renders the R11 layer-pool showcase and asserts four pixel contracts:
//! 1. Background far from any rect is red-dominant (two-phase walker doesn't
//!    corrupt non-layer regions).
//! 2. Middle rect centre shows the 50% Identity-layer blend.
//! 3. Left rect centre is full-opacity white-ish (outside any layer scope).
//! 4. Two pixels straddling the layer clip edge show a clear discontinuity.
//!
//! NOTE on assertion 2's gradient numbers: the plan states the background at
//! x=480 (viewport midpoint) is ≈ (168, 88, 168) sRGB for a red→cyan gradient.
//! A linear red→cyan at 50% x yields ≈ (186, 186, 186), not (168, 88, 168).
//! The plan's value implies non-linear interpolation or different endpoints.
//! This test uses the actual sRGB readback from the existing renderer gradient
//! path at x=480 as the reference value.
//!
//! This file will not compile until R11 lands (`scenes::layer_pool`,
//! `PushLayer`, `PopLayer`, `LayerEffect`). That is expected.
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap, clippy::too_many_arguments, dead_code)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use std::sync::mpsc;

use pollster::FutureExt;
use sigil_renderer::{OffscreenTarget, WgpuRenderer};

// Physical scale factor: texture dims = viewport × SCALE.
// Viewport (960, 320) × scale 1 = (960, 320) px — under the 2048-px downlevel cap.
const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;
const VIEWPORT: (f32, f32) = (960.0, 320.0);

// Locked rect geometry (x, y, w, h):
//   Left   Rect(100, 80, 180, 160)  centre (190, 160)
//   Middle Rect(390, 80, 180, 160)  centre (480, 160) — inside Identity layer
//   Right  Rect(680, 80, 180, 160)
const LEFT_RECT: (f32, f32, f32, f32) = (100.0, 80.0, 180.0, 160.0);
const MID_RECT: (f32, f32, f32, f32) = (390.0, 80.0, 180.0, 160.0);

// 50% blend expectation at middle rect centre. The existing gradient pipeline
// reads back near (174,174,192) under the middle rect, so the 50% layer blend
// lands near (207,207,221).
const BLEND_EXPECT_R: u8 = 204;
const BLEND_EXPECT_G: u8 = 207;
const BLEND_EXPECT_B: u8 = 209;
const BLEND_TOL: u8 = 20;

// ---- entry point ------------------------------------------------------------

/// Render the R11 layer-pool scene and assert all four pixel-level contracts.
/// Skips cleanly when no GPU adapter is available (headless CI).
#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn layer_pixel_check() {
    let Some((device, queue)) = try_request_device() else {
        println!("no adapter — skipping layer_pixel_check");
        return;
    };

    let pixels = render_layer_pool_scene(&device, &queue);
    let phys_w = (VIEWPORT.0 * SCALE as f32) as u32;

    assert_layer_free_content_unchanged(&pixels, phys_w);
    assert_layered_middle_rect_at_half_opacity(&pixels, phys_w);
    assert_unlayered_rects_full_opacity(&pixels, phys_w);
    assert_layer_boundary_sharp(&pixels, phys_w);
}

// ---- render -----------------------------------------------------------------

fn render_layer_pool_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let (command_scene, viewport, clear) = sigil_renderer::scenes::layer_pool();

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

// ---- assertion 1: background unaffected by two-phase walker -----------------

/// At (50, 160) — far from any rect, near the red end of the gradient — R must
/// be > 80 and B must be < 80. The two-phase walker must not corrupt non-layer
/// regions of the frame.
fn assert_layer_free_content_unchanged(pixels: &[u8], phys_w: u32) {
    let x = 50_u32;
    let y = 160_u32;
    let [r, _g, b, _a] = pixel_rgba(pixels, phys_w, x, y);

    assert!(
        r > 80,
        "layer_free_content: at ({x},{y}) R={r}; expected > 80. \
         The two-phase walker may be clearing the main target during Phase 1 \
         snapshot allocation. Check that Phase 1 only reads the command list.",
    );
    assert!(
        b < 80,
        "layer_free_content: at ({x},{y}) B={b}; expected < 80 (red end, not cyan). \
         If B is high, the gradient endpoints may be swapped or the offscreen \
         composite is bleeding into this region.",
    );
}

// ---- assertion 2: middle rect at half opacity via Identity layer -------------

/// At (480, 160) — centre of middle rect inside the Identity layer — the pixel
/// must be the 50% blend of rect fill (240,240,250) and background gradient.
/// Expected ≈ (204, 164, 209) within ±20 per channel.
fn assert_layered_middle_rect_at_half_opacity(pixels: &[u8], phys_w: u32) {
    let cx = (MID_RECT.0 + MID_RECT.2 / 2.0) as u32; // 480
    let cy = (MID_RECT.1 + MID_RECT.3 / 2.0) as u32; // 160
    let [r, g, b, _a] = pixel_rgba(pixels, phys_w, cx, cy);

    assert!(
        abs_diff(r, BLEND_EXPECT_R) <= BLEND_TOL,
        "half_opacity: R at ({cx},{cy})={r}; expected {BLEND_EXPECT_R} ±{BLEND_TOL}. \
         0.5×fill_r(240)+0.5×bg_r(≈174)≈{BLEND_EXPECT_R}. \
         If R≈240 the layer opacity is not applied (check LayerScope::opacity=0.5).",
    );
    assert!(
        abs_diff(g, BLEND_EXPECT_G) <= BLEND_TOL,
        "half_opacity: G at ({cx},{cy})={g}; expected {BLEND_EXPECT_G} ±{BLEND_TOL}. \
         0.5×fill_g(240)+0.5×bg_g(≈174)={BLEND_EXPECT_G}. \
         If G≈240 the composite quad is not modulating by opacity.",
    );
    assert!(
        abs_diff(b, BLEND_EXPECT_B) <= BLEND_TOL,
        "half_opacity: B at ({cx},{cy})={b}; expected {BLEND_EXPECT_B} ±{BLEND_TOL}. \
         0.5×fill_b(250)+0.5×bg_b(≈192)={BLEND_EXPECT_B}. \
         Check that the offscreen texture clears to transparent, not to bg colour.",
    );
}

// ---- assertion 3: left rect at full opacity (outside any layer) -------------

/// At (190, 160) — centre of left rect — all channels must be > 220.
/// The left rect is outside any PushLayer scope; full opacity must be preserved.
fn assert_unlayered_rects_full_opacity(pixels: &[u8], phys_w: u32) {
    let cx = (LEFT_RECT.0 + LEFT_RECT.2 / 2.0) as u32; // 190
    let cy = (LEFT_RECT.1 + LEFT_RECT.3 / 2.0) as u32; // 160
    let [r, g, b, _a] = pixel_rgba(pixels, phys_w, cx, cy);

    assert!(
        r > 220,
        "full_opacity: R at ({cx},{cy})={r}; expected > 220. \
         Left rect fill R=240. If R≤220 the walker applies opacity outside the layer clip.",
    );
    assert!(
        g > 220,
        "full_opacity: G at ({cx},{cy})={g}; expected > 220. \
         Left rect fill G=240. Check PopLayer restores the main render target.",
    );
    assert!(
        b > 220,
        "full_opacity: B at ({cx},{cy})={b}; expected > 220. \
         Left rect fill B=250. If B is low, the composite blit may overflow the clip rect.",
    );
}

// ---- assertion 4: sharp pixel discontinuity at the layer clip edge ----------

/// x=388 (2 px outside middle rect left edge) vs x=392 (2 px inside).
/// At y=160, at least one of |ΔR| or |ΔG| must exceed 30.
/// Proves the Identity layer applies inside the clip, not outside.
fn assert_layer_boundary_sharp(pixels: &[u8], phys_w: u32) {
    let y = 160_u32;
    let x_outside = 388_u32; // 2 px left of clip edge (x=390)
    let x_inside = 392_u32; // 2 px right of clip edge

    let [ro, go, _bo, _ao] = pixel_rgba(pixels, phys_w, x_outside, y);
    let [ri, gi, _bi, _ai] = pixel_rgba(pixels, phys_w, x_inside, y);

    let delta_r = abs_diff(ro, ri);
    let delta_g = abs_diff(go, gi);

    assert!(
        delta_r > 30 || delta_g > 30,
        "layer_boundary: no sharp discontinuity at Identity layer left edge (x=390). \
         Outside ({x_outside},{y}): R={ro} G={go}. Inside ({x_inside},{y}): R={ri} G={gi}. \
         ΔR={delta_r}, ΔG={delta_g}; expected at least one > 30. \
         Check that LayerScope::clip is used as the scissor rect for the composite quad.",
    );
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
            apply_limit_buckets: false,
        })
        .block_on()
        .ok()?;
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("layer-pixel-check device"),
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
        label: Some("layer-pixel-check capture"),
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
        label: Some("layer-pixel-check readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("layer-pixel-check copy encoder"),
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
