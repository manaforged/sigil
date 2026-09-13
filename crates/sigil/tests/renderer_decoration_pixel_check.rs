#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_lossless, clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap, clippy::too_many_arguments, dead_code)]
#![allow(clippy::doc_markdown, clippy::missing_const_for_fn)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use std::sync::mpsc;

use pollster::FutureExt;
use sigil::renderer::{OffscreenTarget, WgpuRenderer};

const SCALE: u32 = 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

const UNDERLINE_MID_Y: u32 = 43;
const STRIKETHROUGH_MID_Y: u32 = 102;
const WAVY_BAND_Y0: u32 = 160;
const WAVY_BAND_Y1: u32 = 175;

const SCAN_X0: u32 = 440;
const SCAN_LEN: u32 = 32;

const PERIOD_PX: u32 = 32;
const COL_P0: u32 = 80;
const COL_P_HALF: u32 = COL_P0 + PERIOD_PX / 2;
const COL_P1: u32 = COL_P0 + PERIOD_PX;

const WAVE_LINE_WIDTH: f32 = 2.5;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn decoration_pixels_match_underline_strikethrough_and_wave_contracts() {
    let Some((device, queue)) = try_request_device() else {
        println!("no adapter — skipping decoration_pixel_check");
        return;
    };

    let pixels = render_decoration_scene(&device, &queue);
    let phys_w = (800_f32 * SCALE as f32) as u32;

    assert_underline_bar_is_cyan(&pixels, phys_w);
    assert_strikethrough_bar_is_red(&pixels, phys_w);
    assert_wavy_underline_oscillates(&pixels, phys_w);
    assert_wave_period_is_32px(&pixels, phys_w);
}

fn render_decoration_scene(device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    let (command_scene, viewport, clear) = sigil::renderer::scenes::decoration();

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

fn assert_underline_bar_is_cyan(pixels: &[u8], phys_w: u32) {
    let y = UNDERLINE_MID_Y;
    let hits = count_channel_above(pixels, phys_w, SCAN_X0, y, SCAN_LEN, Channel::B, 200);
    let threshold = (SCAN_LEN * 9) / 10;

    assert!(
        hits >= threshold,
        "underline: at y={y} x=[{SCAN_X0},{}) expected ≥ {threshold}/32 pixels \
         with B > 200 (cyan underline). Got {hits}. \
         Color is Rgba8(100,200,255,255); B channel must dominate after sRGB \
         compositing. Check decoration_paint.wgsl DECORATION_UNDERLINE branch \
         and per-instance color packing.",
        SCAN_X0 + SCAN_LEN,
    );
}

fn assert_strikethrough_bar_is_red(pixels: &[u8], phys_w: u32) {
    let y = STRIKETHROUGH_MID_Y;
    let hits = count_channel_above(pixels, phys_w, SCAN_X0, y, SCAN_LEN, Channel::R, 200);
    let threshold = (SCAN_LEN * 9) / 10;

    assert!(
        hits >= threshold,
        "strikethrough: at y={y} x=[{SCAN_X0},{}) expected ≥ {threshold}/32 pixels \
         with R > 200 (red strikethrough). Got {hits}. \
         Color is Rgba8(220,60,60,255); R channel must dominate after compositing. \
         Check decoration_paint.wgsl DECORATION_STRIKETHROUGH branch.",
        SCAN_X0 + SCAN_LEN,
    );
}

fn assert_wavy_underline_oscillates(pixels: &[u8], phys_w: u32) {
    let x = COL_P1;
    let mut orange_count = 0u32;
    let mut background_count = 0u32;

    for y in WAVY_BAND_Y0..=WAVY_BAND_Y1 {
        let r = pixel_channel(pixels, phys_w, x, y, Channel::R);
        if r > 180 {
            orange_count += 1;
        } else if r < 80 {
            background_count += 1;
        }
    }

    assert!(
        orange_count >= 2,
        "wavy underline oscillates: vertical column at x={x} y=[{WAVY_BAND_Y0},{WAVY_BAND_Y1}] \
         found only {orange_count} orange-tinted pixels (R > 180). Expected ≥ 2. \
         The sine wave must intersect this column. Check decoration_paint.wgsl \
         DECORATION_WAVY_UNDERLINE branch: wave_y = centre_y + sin(phase) * amplitude. \
         Verify period=32.0 and line_width=2.5 are passed to the shader.",
    );

    assert!(
        background_count >= 2,
        "wavy underline oscillates: vertical column at x={x} y=[{WAVY_BAND_Y0},{WAVY_BAND_Y1}] \
         found only {background_count} background pixels (R < 80). Expected ≥ 2. \
         The sine wave must NOT fill the entire column — if it does, the wave amplitude \
         is too large or the band is rendering as a solid fill. \
         Check amplitude = line_width * 0.75 in DECORATION_WAVY_UNDERLINE.",
    );
}

fn assert_wave_period_is_32px(pixels: &[u8], phys_w: u32) {
    let peak_p0 = wave_peak_y(pixels, phys_w, COL_P0);
    let peak_half = wave_peak_y(pixels, phys_w, COL_P_HALF);
    let peak_p1 = wave_peak_y(pixels, phys_w, COL_P1);

    let tol = WAVE_LINE_WIDTH as i32;

    let same_phase_diff = (peak_p0 as i32 - peak_p1 as i32).abs();
    assert!(
        same_phase_diff <= tol,
        "wave period: peak-y at x={COL_P0} ({peak_p0}) and x={COL_P1} ({peak_p1}) \
         are one full period (32 px) apart and must match within ±{tol} px. \
         Diff = {same_phase_diff}. Check that phase = local.x / in.period * TAU in WGSL \
         and that the period uniform is set to 32.0 logical px.",
    );

    let half_phase_diff = (peak_p0 as i32 - peak_half as i32).abs();
    assert!(
        half_phase_diff >= tol,
        "wave period: peak-y at x={COL_P0} ({peak_p0}) and x={COL_P_HALF} ({peak_half}) \
         are half a period apart — the wave must have moved by ≥ {tol} px. \
         Diff = {half_phase_diff}. If diff < {tol} the wave may not be oscillating \
         (flat fill, zero amplitude, or wrong period). \
         Check amplitude = line_width * 0.75 = 1.875 px and period = 32.0 in the shader.",
    );
}

#[derive(Clone, Copy)]
enum Channel {
    R,
    G,
    B,
}

fn pixel_channel(pixels: &[u8], width: u32, x: u32, y: u32, ch: Channel) -> u8 {
    let base = ((y * width + x) * BYTES_PER_PIXEL) as usize;
    match ch {
        Channel::R => pixels[base],
        Channel::G => pixels[base + 1],
        Channel::B => pixels[base + 2],
    }
}

fn count_channel_above(
    pixels: &[u8],
    width: u32,
    x0: u32,
    y: u32,
    len: u32,
    ch: Channel,
    threshold: u8,
) -> u32 {
    (x0..x0 + len)
        .filter(|&x| pixel_channel(pixels, width, x, y, ch) > threshold)
        .count() as u32
}

fn wave_peak_y(pixels: &[u8], width: u32, x: u32) -> u32 {
    (WAVY_BAND_Y0..=WAVY_BAND_Y1)
        .max_by_key(|&y| pixel_channel(pixels, width, x, y, Channel::R))
        .expect("band has rows")
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
            label: Some("decoration-pixel-check device"),
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
        label: Some("decoration-pixel-check capture"),
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
        label: Some("decoration-pixel-check readback"),
        size: u64::from(padded_bpr) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("decoration-pixel-check copy encoder"),
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
