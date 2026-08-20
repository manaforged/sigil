//! `WgpuRenderer::record_screen` headless tests.
//!
//! `record_screen` records a screen-space scene into a caller-owned encoder
//! without submitting (the host submits, ordering UI after its own passes in
//! one submission — DR-054). These tests prove two contracts against a
//! headless offscreen target:
//!
//! 1. **Ordering** — a scene that draws a red full-viewport rect then a blue
//!    half rect composites blue *over* red in the blue region, and leaves red
//!    in the bottom region. (The pre-fix shared-buffer bug made every batched
//!    draw read the last write, so only one rect would survive.)
//! 2. **Parity** — `record_screen` + one `queue.submit` produces the same
//!    pixels as the self-submitting `submit_screen` for the same scene.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(
    clippy::cast_precision_loss,
    clippy::print_stdout,
    clippy::missing_const_for_fn
)]

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
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use std::sync::mpsc;

use pollster::FutureExt as _;
use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, CornerRadii, SceneBuilder, SceneRectStyle, ScreenSubmission, WgpuRenderer,
};
use sigil_style::Rgba8;

const W: u32 = 64;
const H: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (W as f32, H as f32);

/// Red full-viewport rect, then a blue rect over the top half.
fn ordering_scene() -> CommandScene {
    let mut b = SceneBuilder::new();
    b.draw_rect(
        Rect::new(Point::new(0.0, 0.0), W as f32, H as f32),
        solid(220, 30, 30),
    );
    b.draw_rect(
        Rect::new(Point::new(0.0, 0.0), W as f32, (H / 2) as f32),
        solid(30, 30, 220),
    );
    b.finish()
}

#[test]
fn record_screen_orders_draws_and_matches_submit_screen() {
    let Some((device, queue)) = try_device() else {
        println!("no adapter — skipping record_screen test");
        return;
    };
    let scene = ordering_scene();

    // Path A: record into a caller-owned encoder, submit once.
    let recorded = {
        let tex = capture_texture(&device);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("record_screen test encoder"),
        });
        WgpuRenderer::new()
            .expect("renderer")
            .record_screen(&submission(&device, &queue, &view), &scene, &mut enc)
            .expect("record_screen");
        queue.submit(core::iter::once(enc.finish()));
        read_pixels(&device, &queue, &tex)
    };

    // Ordering: top half is blue (drawn second), bottom half is red.
    let top = px(&recorded, W / 2, H / 4);
    let bottom = px(&recorded, W / 2, 3 * H / 4);
    assert!(
        top[2] > 150 && top[0] < 90,
        "top region should be blue (B high, R low); got {top:?}"
    );
    assert!(
        bottom[0] > 150 && bottom[2] < 90,
        "bottom region should be red (R high, B low); got {bottom:?}"
    );

    // Path B: self-submitting submit_screen must produce the same pixels.
    let submitted = {
        let tex = capture_texture(&device);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        WgpuRenderer::new()
            .expect("renderer")
            .submit_screen(&submission(&device, &queue, &view), &scene)
            .expect("submit_screen");
        read_pixels(&device, &queue, &tex)
    };
    for (x, y) in [(W / 2, H / 4), (W / 2, 3 * H / 4), (4, 4)] {
        let a = px(&recorded, x, y);
        let b = px(&submitted, x, y);
        for ch in 0..4 {
            let diff = i32::from(a[ch]) - i32::from(b[ch]);
            assert!(
                diff.abs() <= 2,
                "record_screen vs submit_screen mismatch at ({x},{y}) ch {ch}: \
                 {a:?} vs {b:?}"
            );
        }
    }
}

// ---- helpers ----------------------------------------------------------------

const fn solid(r: u8, g: u8, b: u8) -> SceneRectStyle {
    SceneRectStyle::new(Rgba8 { r, g, b, a: 255 }, CornerRadii::ZERO, 10_000)
}

fn submission<'a>(
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    view: &'a wgpu::TextureView,
) -> ScreenSubmission<'a> {
    ScreenSubmission {
        device,
        queue,
        view,
        format: FORMAT,
        viewport: VIEWPORT,
        scale: 1.0,
        clear: Some(Color::BLACK),
    }
}

fn try_device() -> Option<(wgpu::Device, wgpu::Queue)> {
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
            label: Some("record_screen-test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .block_on()
        .ok()
}

fn capture_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("record_screen capture"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
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

fn px(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

fn align_up(value: u32, align: u32) -> u32 {
    value.div_ceil(align) * align
}

fn read_pixels(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let bpr = align_up(W * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("record_screen readback"),
        size: u64::from(bpr) * u64::from(H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("record_screen copy"),
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
                bytes_per_row: Some(bpr),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
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
        .expect("poll");
    rx.recv().expect("recv").expect("map ok");
    let mapped = slice.get_mapped_range();
    let mut out = Vec::with_capacity((W * H * 4) as usize);
    for row in 0..H {
        let off = (row * bpr) as usize;
        out.extend_from_slice(&mapped[off..off + (W * 4) as usize]);
    }
    out
}
