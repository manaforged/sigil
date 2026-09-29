#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

#[path = "scene_support/mod.rs"]
mod scene_support;

use scene_support::{color, rect};
use sigil::geometry::Point;
use sigil::renderer::{OffscreenTarget, WgpuRenderer};
use sigil::scene::{
    CommandScene, CornerRadii, GradientStop, LinearGradient, SceneBuilder, SceneGradientStyle,
    SceneRectStyle,
};
use sigil::style::Color;
use sigil_kittest::gpu::read_pixels;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (256.0, 256.0);
const DRAWS: u32 = 2_100;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn pass_heavy_scene_keeps_device_alive() {
    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        return;
    };
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();

    let scene = pass_heavy_scene();
    let pixels = render_offscreen(&mut renderer, &device, &queue, &scene);

    assert_eq!(pixel(&pixels, 128, 150), [240, 30, 30, 255], "red row lost");
    for x in 1..255 {
        let [r, g, b, a] = pixel(&pixels, x, 151);
        assert!(
            (24..=35).contains(&r),
            "red channel outside gradient dither bounds: {r}"
        );
        assert_eq!(g, r, "equal gradient channels diverged");
        assert!((239..=241).contains(&b), "blue row lost: {b}");
        assert_eq!(a, 255, "blue row lost opacity");
    }
}

fn pass_heavy_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    for row in 0..DRAWS {
        let rect = rect(0.0, row as f32, VIEWPORT.0, 1.0);
        if row % 2 == 0 {
            builder.draw_rect(
                rect,
                SceneRectStyle::new(color(240, 30, 30, 255), CornerRadii::ZERO, u16::MAX),
            );
        } else {
            builder.draw_gradient(
                rect,
                SceneGradientStyle::new(
                    LinearGradient::new(
                        Point::new(0.0, 0.0),
                        Point::new(VIEWPORT.0, 0.0),
                        vec![
                            GradientStop::new(0.0, color(30, 30, 240, 255)),
                            GradientStop::new(1.0, color(30, 30, 240, 255)),
                        ],
                    ),
                    CornerRadii::ZERO,
                    u16::MAX,
                ),
            );
        }
    }
    builder.finish()
}

fn render_offscreen(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
) -> Vec<u8> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tall-scene target"),
        size: wgpu::Extent3d {
            width: VIEWPORT.0 as u32,
            height: VIEWPORT.1 as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: VIEWPORT,
                scale: 1.0,
            },
            scene,
            Color::BLACK,
        )
        .expect("offscreen render");
    read_pixels(
        device,
        queue,
        &texture,
        VIEWPORT.0 as u32,
        VIEWPORT.1 as u32,
    )
    .expect("sigil-kittest")
}

const fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * VIEWPORT.0 as u32 + x) * 4) as usize;
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}
