#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

use std::sync::mpsc;

use sigil::hal::{Color, Point, Rect};
use sigil::renderer::{
    CommandScene, CornerRadii, GradientStop, LinearGradient, OffscreenTarget, SceneBuilder,
    SceneGradientStyle, SceneRectStyle, WgpuRenderer,
};
use sigil::style::Rgba8;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (256.0, 256.0);
const DRAWS: u32 = 2_100;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn pass_heavy_scene_keeps_device_alive() {
    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device - skipping tall scene test: {err}");
        return;
    }
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();

    let scene = pass_heavy_scene();
    let pixels = render_offscreen(&mut renderer, &device, &queue, &scene);

    assert_eq!(pixel(&pixels, 128, 150), [240, 30, 30, 255], "red row lost");
    assert_eq!(
        pixel(&pixels, 128, 151),
        [30, 30, 240, 255],
        "blue row lost"
    );
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
    read_pixels(device, queue, &texture)
}

fn read_pixels(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let width = VIEWPORT.0 as u32;
    let height = VIEWPORT.1 as u32;
    let bytes_per_row = width * 4;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("capture readback"),
        size: u64::from(bytes_per_row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(core::iter::once(encoder.finish()));
    let slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    receiver
        .recv()
        .expect("map callback")
        .expect("readback map — device lost");
    slice.get_mapped_range().expect("mapped range").to_vec()
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

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
