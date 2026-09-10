#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::unnecessary_cast,
    dead_code
)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use std::sync::mpsc;

use pollster::FutureExt;
use sigil_renderer::{OffscreenTarget, WgpuRenderer};

const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

const BAR_SCAN_RECT0_X: u32 = 216;
const BAR_SCAN_RECT1_X: u32 = 520;
const BAR_SCAN_RECT2_X: u32 = 824;
const BAR_SCAN_LEN: u32 = 8;

const CLIP_SCAN_START_X: u32 = 50;
const CLIP_SCAN_LEN: u32 = 16;
const CLIP_SCAN_Y: u32 = 180;

const RECT0_CENTRE_X: u32 = 220;
const RECT1_CENTRE_X: u32 = 520;
const RECT2_CENTRE_X: u32 = 820;
const RECT_CENTRE_Y: u32 = 180;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn blur_pixel_check() {
    let (device, queue) = try_request_device().expect(
        "no wgpu adapter — blur_pixel_check needs a real GPU (run the GPU tier with --ignored)",
    );

    let pixels = render_backdrop_blur_scene(&device, &queue);
    let phys_w = (1024_u32 * SCALE) as u32;

    assert_control_preserves_contrast(&pixels, phys_w);
    assert_mild_blur_reduces_contrast(&pixels, phys_w);
    assert_strong_blur_eliminates_contrast(&pixels, phys_w);
    assert_blur_respects_layer_clip(&pixels, phys_w);
}

fn render_backdrop_blur_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let (command_scene, viewport, clear) = sigil_renderer::scenes::backdrop_blur();

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

fn assert_control_preserves_contrast(pixels: &[u8], phys_w: u32) {
    let y = RECT_CENTRE_Y;
    let r_values = scan_r_channel(pixels, phys_w, BAR_SCAN_RECT0_X, y, BAR_SCAN_LEN);

    let min_r = *r_values.iter().min().unwrap();
    let max_r = *r_values.iter().max().unwrap();

    assert!(
        min_r < 50,
        "control_preserves_contrast: min_R across x=[{BAR_SCAN_RECT0_X}..{}] y={y} \
         is {min_r}; expected < 50. Bar k=13 (x=208..215) should be black (R≈0) \
         when sigma=0. If min_R≥50, the sigma=0 path is still blurring. \
         Check that LayerEffect::LayerBlur {{ sigma: 0.0 }} triggers 0 Kawase \
         passes (pass-through composite only). Scanned R values: {r_values:?}",
        BAR_SCAN_RECT0_X + BAR_SCAN_LEN - 1,
    );

    assert!(
        max_r > 200,
        "control_preserves_contrast: max_R across x=[{BAR_SCAN_RECT0_X}..{}] y={y} \
         is {max_r}; expected > 200. The gap adjacent to bar k=13 should be white \
         (R≈255) when no blur is applied. If max_R≤200, either the bar geometry \
         doesn't reach this x range or the background is darker than expected. \
         Confirm 24 bars at 16-px period, 8 px wide, starting at x=0. \
         Scanned R values: {r_values:?}",
        BAR_SCAN_RECT0_X + BAR_SCAN_LEN - 1,
    );
}

fn assert_mild_blur_reduces_contrast(pixels: &[u8], phys_w: u32) {
    let y = RECT_CENTRE_Y;
    let r_values = scan_r_channel(pixels, phys_w, BAR_SCAN_RECT1_X, y, BAR_SCAN_LEN);
    let intermediate = r_values.iter().filter(|&&r| r > 50 && r < 200).count();

    assert!(
        intermediate >= 3,
        "mild_blur_reduces_contrast: only {intermediate} pixels in [50,200] across \
         x=[{BAR_SCAN_RECT1_X}..{}] y={y} inside rect 1 (sigma=6). \
         An unblurred bar/gap edge has 0 intermediates; sigma=6 dual-Kawase must \
         smooth at least 3 pixels through the mid-grey band. \
         Scanned R values: {r_values:?}",
        BAR_SCAN_RECT1_X + BAR_SCAN_LEN - 1,
    );
}

fn assert_strong_blur_eliminates_contrast(pixels: &[u8], phys_w: u32) {
    let y = RECT_CENTRE_Y;
    let r_values = scan_r_channel(pixels, phys_w, BAR_SCAN_RECT2_X, y, BAR_SCAN_LEN);

    let min_r = *r_values.iter().min().unwrap();
    let max_r = *r_values.iter().max().unwrap();
    let contrast_window = max_r.saturating_sub(min_r);

    assert!(
        contrast_window < 80,
        "strong_blur_eliminates_contrast: R contrast window (max-min) across \
         x=[{BAR_SCAN_RECT2_X}..{}] y={y} inside rect 2 (sigma=18) is \
         {contrast_window} (min={min_r}, max={max_r}); expected < 80. \
         Six Kawase passes (3 down + 3 up) should smear 8-px bars into near-uniform \
         gray. If contrast_window≥80, check: (a) pass count for sigma=18 in \
         blur_paint.rs — sigma in (16,32] → 6 passes; (b) that the pyramid textures \
         are not being reused across frames without clearing; (c) that the final \
         upsample composites back to the layer rect, not the full viewport. \
         Scanned R values: {r_values:?}",
        BAR_SCAN_RECT2_X + BAR_SCAN_LEN - 1,
    );
}

fn assert_blur_respects_layer_clip(pixels: &[u8], phys_w: u32) {
    let y = CLIP_SCAN_Y;
    let r_values = scan_r_channel(pixels, phys_w, CLIP_SCAN_START_X, y, CLIP_SCAN_LEN);

    let dark_count = r_values.iter().filter(|&&r| r < 50).count();
    let light_count = r_values.iter().filter(|&&r| r > 200).count();

    assert!(
        dark_count >= 3,
        "blur_respects_layer_clip: only {dark_count} pixels with R<50 (bar) found \
         across x=[{CLIP_SCAN_START_X}..{}] y={y}; expected ≥3. \
         This region is outside all blur surfaces (nearest is rect 0 left edge at x=100). \
         Bars k=3 (x=48..55) and k=4 (x=64..71) span this scan. \
         If dark pixels are missing the blur is overwriting the background outside \
         its clip rect. Confirm composite quad scissors to LayerScope::clip. \
         Scanned R values: {r_values:?}",
        CLIP_SCAN_START_X + CLIP_SCAN_LEN - 1,
    );

    assert!(
        light_count >= 3,
        "blur_respects_layer_clip: only {light_count} pixels with R>200 (gap) found \
         across x=[{CLIP_SCAN_START_X}..{}] y={y}; expected ≥3. \
         Gap k=3 (x=56..63) lies inside this scan and should read white (R≈255). \
         If light pixels are absent, check that the bar background is actually \
         rendering a white gap between the black bars and that the bar count/period \
         (24 bars, 16-px period, 8-px width) is implemented correctly in \
         scenes::backdrop_blur(). Scanned R values: {r_values:?}",
        CLIP_SCAN_START_X + CLIP_SCAN_LEN - 1,
    );
}

fn scan_r_channel(pixels: &[u8], phys_w: u32, x_start: u32, y: u32, len: u32) -> Vec<u8> {
    (x_start..x_start + len)
        .map(|x| pixel_rgba(pixels, phys_w, x, y)[0])
        .collect()
}

fn pixel_rgba(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let base = ((y * width + x) * BYTES_PER_PIXEL) as usize;
    [
        pixels[base],
        pixels[base + 1],
        pixels[base + 2],
        pixels[base + 3],
    ]
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
            label: Some("blur-pixel-check device"),
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
        label: Some("blur-pixel-check capture"),
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
        label: Some("blur-pixel-check readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("blur-pixel-check copy encoder"),
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
