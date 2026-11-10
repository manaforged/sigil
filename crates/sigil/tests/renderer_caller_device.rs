#![cfg(feature = "kittest")]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

use crate::scene_support;

use scene_support::{color, rect};
use sigil::renderer::{OffscreenTarget, ScreenSubmission, WgpuRenderer};
use sigil::scene::{CommandScene, DrawRect, SceneBuilder};
use sigil::style::Color;
use sigil_kittest::gpu::read_pixels;

const W: u32 = 64;
const H: u32 = 64;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (W as f32, H as f32);

fn ordering_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.draw_rect(
        DrawRect::new(
            rect(0.0, 0.0, VIEWPORT.0, VIEWPORT.1),
            color(220, 30, 30, 255),
        )
        .opacity(10_000),
    );
    builder.draw_rect(
        DrawRect::new(
            rect(0.0, 0.0, VIEWPORT.0, VIEWPORT.1 / 2.0),
            color(30, 30, 220, 255),
        )
        .opacity(10_000),
    );
    builder.finish()
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn caller_owned_device_does_not_initialize_a_second_device() {
    let Some((device, queue)) = gpu() else {
        return;
    };
    let texture = capture_texture(&device);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut renderer = WgpuRenderer::new();

    renderer
        .submit_screen(&submission(&device, &queue, &view), &ordering_scene())
        .expect("submit_screen");

    assert!(
        renderer.device().is_none(),
        "a caller-owned device must not initialize a redundant renderer device"
    );
    let pixels = read_pixels(&device, &queue, &texture, W, H).expect("sigil-kittest");
    assert_eq!(px(&pixels, W / 2, H / 4), [30, 30, 220, 255]);
    assert_eq!(px(&pixels, W / 2, 3 * H / 4), [220, 30, 30, 255]);
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn offscreen_capture_uses_the_callers_device_and_preserves_draw_order() {
    let Some((device, queue)) = gpu() else {
        return;
    };
    let texture = capture_texture(&device);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut renderer = WgpuRenderer::new();
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
    let pixels = read_pixels(&device, &queue, &texture, W, H).expect("sigil-kittest");
    assert_eq!(px(&pixels, W / 2, H / 4), [30, 30, 220, 255]);
    assert_eq!(px(&pixels, W / 2, 3 * H / 4), [220, 30, 30, 255]);
}

const fn submission<'a>(
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

fn capture_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("caller device capture"),
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

const fn px(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .cloned()
}
