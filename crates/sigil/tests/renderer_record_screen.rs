#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(
    clippy::cast_precision_loss,
    clippy::print_stdout,
    clippy::missing_const_for_fn
)]

use sigil::hal::{Color, Point, Rect};
use sigil::renderer::{
    CommandScene, CornerRadii, OffscreenTarget, SceneBuilder, SceneRectStyle, ScreenSubmission,
    WgpuRenderer,
};
use sigil::style::Rgba8;
use sigil_kittest::gpu::read_pixels;

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
    let Some((device, queue)) = gpu() else {
        return;
    };
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
        read_pixels(&device, &queue, &tex, W, H).expect("sigil-kittest")
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
        read_pixels(&device, &queue, &tex, W, H).expect("sigil-kittest")
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
    let Some((device, queue)) = gpu() else {
        return;
    };
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
    let Some((device, queue)) = gpu() else {
        return;
    };
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
    let pixels = read_pixels(&device, &queue, &texture, W, H).expect("sigil-kittest");
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

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    sigil_kittest::gpu::shared_device()
        .expect("sigil-kittest")
        .cloned()
}
