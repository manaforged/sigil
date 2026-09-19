#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil::hal::{Color, Point, Rect};
use sigil::renderer::{OffscreenTarget, SceneBuilder, SceneRectStyle, WgpuRenderer};
use sigil::style::Rgba8;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn ordered_batches_share_one_rect_upload_buffer() {
    let mut renderer = WgpuRenderer::new().expect("renderer");
    renderer
        .ensure_headless_for_tests()
        .expect("GPU device required");
    let device = renderer.device().expect("device").clone();
    let queue = renderer.queue().expect("queue").clone();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame-upload target"),
        size: wgpu::Extent3d {
            width: 96,
            height: 48,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    renderer
        .render_command_scene_offscreen(
            OffscreenTarget {
                device: &device,
                queue: &queue,
                view: &view,
                format: FORMAT,
                viewport: (96.0, 48.0),
                scale: 1.0,
            },
            &two_batch_scene(),
            Color::BLACK,
        )
        .expect("render");

    let stats = renderer.last_frame_stats();
    assert_eq!(stats.rectangle_batches, 2);
    assert_eq!(stats.rectangle_instances, 2);
    assert_eq!(
        stats.frame_upload_buffers, 2,
        "one rect buffer + one uniform"
    );
    assert_eq!(stats.frame_upload_writes, 3, "two ranges + one uniform");
    assert_eq!(
        stats.frame_uploaded_bytes, stats.frame_upload_capacity_bytes,
        "both reserved rect slots and the shared uniform must be populated"
    );
}

fn two_batch_scene() -> sigil::renderer::CommandScene {
    let mut scene = SceneBuilder::new();
    scene.draw_rect(rect(4.0, 4.0), SceneRectStyle::solid(color(220, 60, 80)));
    scene.with_clip(Rect::new(Point::new(48.0, 0.0), 48.0, 48.0), |scene| {
        scene.draw_rect(rect(52.0, 4.0), SceneRectStyle::solid(color(70, 160, 240)));
    });
    scene.finish()
}

const fn rect(x: f32, y: f32) -> Rect {
    Rect::new(Point::new(x, y), 40.0, 40.0)
}

const fn color(r: u8, g: u8, b: u8) -> Rgba8 {
    Rgba8 { r, g, b, a: 255 }
}
