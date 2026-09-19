#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(
    clippy::cast_precision_loss,
    clippy::print_stdout,
    clippy::missing_const_for_fn
)]

use std::sync::mpsc;

use pollster::FutureExt;
use sigil::hal::{Color, Point, Rect};
use sigil::renderer::{
    CommandScene, CornerRadii, OffscreenTarget, SceneBuilder, SceneRectStyle, ScreenSubmission,
    WgpuRenderer,
};
use sigil::style::Rgba8;

const W: u32 = 64;
const H: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (W as f32, H as f32);

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
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn record_screen_orders_draws_and_matches_submit_screen() {
    let (device, queue) = try_device().expect("GPU device required");
    let scene = ordering_scene();

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

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn caller_owned_device_does_not_initialize_a_second_device() {
    let (device, queue) = try_device().expect("GPU device required");
    let texture = capture_texture(&device);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut renderer = WgpuRenderer::new().expect("renderer");

    renderer
        .submit_screen(&submission(&device, &queue, &view), &ordering_scene())
        .expect("submit_screen");

    assert!(
        renderer.device().is_none(),
        "a caller-owned device must not initialize a redundant renderer device"
    );
}

#[test]
#[ignore = "requires a GPU adapter"]
fn offscreen_capture_uses_the_callers_device_and_preserves_draw_order() {
    let (device, queue) = try_device().expect("GPU device required");
    let texture = capture_texture(&device);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut renderer = WgpuRenderer::new().expect("renderer");
    let target = submission(&device, &queue, &view);
    renderer
        .render_command_scene_offscreen(
            OffscreenTarget::from(&target),
            &ordering_scene(),
            Color::BLACK,
        )
        .expect("offscreen capture");
    assert!(
        renderer.device().is_none(),
        "capture allocated a second device"
    );
    let pixels = read_pixels(&device, &queue, &texture);
    assert_eq!(px(&pixels, W / 2, H / 4), [30, 30, 220, 255]);
    assert_eq!(px(&pixels, W / 2, 3 * H / 4), [220, 30, 30, 255]);
}

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
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
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
    let mapped = slice.get_mapped_range().expect("mapped range");
    let mut out = Vec::with_capacity((W * H * 4) as usize);
    for row in 0..H {
        let off = (row * bpr) as usize;
        out.extend_from_slice(&mapped[off..off + (W * 4) as usize]);
    }
    out
}
