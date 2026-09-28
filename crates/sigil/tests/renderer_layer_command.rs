#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

#[path = "scene_support/mod.rs"]
mod scene_support;

use scene_support::{color, rect};
use sigil::hal::Color;
use sigil::renderer::{OffscreenTarget, WgpuRenderer};
use sigil::scene::{
    CommandScene, CornerRadii, LayerEffect, LayerScope, SceneBuilder, SceneCommand, SceneRectStyle,
};
use sigil_kittest::gpu::read_pixels;
use sigil_kittest::gpu::{Capture, render_capture};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (220.0, 120.0);
const VIEWPORT_WIDTH: u32 = 220;
const VIEWPORT_HEIGHT: u32 = 120;
const CAPTURE: Capture = Capture {
    viewport: VIEWPORT,
    size_px: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
    scale: 1.0,
    clear: Color::rgba(0.05, 0.06, 0.09, 1.0),
};

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn layer_commands_walk_and_initialize_layer_pool() {
    let scene = layer_scene();
    assert_layer_command_shape(&scene);

    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        return;
    };

    let ready_device = renderer.device().expect("headless device").clone();
    let ready_queue = renderer.queue().expect("headless queue").clone();
    render_capture(
        &mut renderer,
        (&ready_device, &ready_queue),
        &scene,
        &CAPTURE,
    )
    .expect("render_capture");

    let (scene_pipelines, _, _, _, _) = renderer
        .pipeline_cache_counts()
        .expect("renderer should have headless cache state");
    assert!(scene_pipelines > 0);
    assert!(renderer.layer_pool_ready_for_tests());
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn rounded_layer_clip_masks_every_descendant_pipeline() {
    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        return;
    };
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();
    let pixels = render_scene_pixels(&mut renderer, &device, &queue, &rounded_clip_scene());

    let corner = pixel(&pixels, 42, 26);
    let edge = pixel(&pixels, 88, 26);
    let centre = pixel(&pixels, 88, 56);
    assert!(corner[0] < 20, "rounded corner leaked fill: {corner:?}");
    assert!(edge[0] > 200, "top edge lost fill: {edge:?}");
    assert!(centre[0] > 200, "clip centre lost fill: {centre:?}");
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn nested_layer_composites_in_parent_local_coordinates() {
    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        return;
    };
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();
    let pixels = render_scene_pixels(&mut renderer, &device, &queue, &nested_layer_scene());

    assert!(
        pixel(&pixels, 90, 50)[1] > 200,
        "nested layer shifted right"
    );
    assert!(
        pixel(&pixels, 150, 50)[1] < 20,
        "nested layer exceeded its clip"
    );
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn retina_layer_allocates_physical_not_logical_pixels() {
    let Some(mut renderer) = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest") else {
        return;
    };
    let device = renderer.device().expect("headless device").clone();
    let queue = renderer.queue().expect("headless queue").clone();
    render_scene_at_scale(&mut renderer, &device, &queue, &rounded_clip_scene(), 2);

    let stats = renderer.last_frame_stats();
    assert_eq!(stats.offscreen_layers, 1);
    assert_eq!(stats.offscreen_pixels, 192 * 128);
}

fn assert_layer_command_shape(scene: &CommandScene) {
    let [
        SceneCommand::PushLayer(scope),
        SceneCommand::Rect(draw),
        SceneCommand::PopLayer,
    ] = scene.commands.as_slice()
    else {
        panic!("expected push-layer, draw-rect, pop-layer");
    };
    assert!((scope.opacity - 0.5).abs() < f32::EPSILON);
    assert_eq!(scope.effect, LayerEffect::Identity);
    assert_eq!(draw.rect, rect(40.0, 24.0, 96.0, 64.0));
}

fn layer_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.push_layer(LayerScope {
        clip: rect(40.0, 24.0, 96.0, 64.0),
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Identity,
        opacity: 0.5,
    });
    builder.draw_rect(
        rect(40.0, 24.0, 96.0, 64.0),
        SceneRectStyle::new(
            color(240, 240, 250, 255),
            CornerRadii::uniform(12.0),
            10_000,
        ),
    );
    builder.pop_layer();
    builder.finish()
}

fn rounded_clip_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.with_composite_layer(
        LayerScope {
            clip: rect(40.0, 24.0, 96.0, 64.0),
            clip_radii: CornerRadii::uniform(24.0),
            effect: LayerEffect::Identity,
            opacity: 1.0,
        },
        |scene| {
            scene.draw_rect(
                rect(20.0, 4.0, 136.0, 104.0),
                SceneRectStyle::solid(color(240, 30, 30, 255)),
            );
        },
    );
    builder.finish()
}

fn nested_layer_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    let plain = |clip| LayerScope {
        clip,
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Identity,
        opacity: 1.0,
    };
    builder.with_composite_layer(plain(rect(30.0, 10.0, 160.0, 100.0)), |scene| {
        scene.with_composite_layer(plain(rect(80.0, 35.0, 60.0, 50.0)), |scene| {
            scene.draw_rect(
                rect(80.0, 35.0, 60.0, 50.0),
                SceneRectStyle::solid(color(30, 240, 30, 255)),
            );
        });
    });
    builder.finish()
}

fn render_scene_pixels(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
) -> Vec<u8> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rounded-layer capture"),
        size: wgpu::Extent3d {
            width: VIEWPORT_WIDTH,
            height: VIEWPORT_HEIGHT,
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
            Color::rgba(0.0, 0.0, 0.0, 1.0),
        )
        .expect("render rounded clip");
    read_pixels(device, queue, &texture, VIEWPORT_WIDTH, VIEWPORT_HEIGHT).expect("sigil-kittest")
}

fn render_scene_at_scale(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
    scale: u16,
) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scaled-layer target"),
        size: wgpu::Extent3d {
            width: VIEWPORT_WIDTH * u32::from(scale),
            height: VIEWPORT_HEIGHT * u32::from(scale),
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
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: VIEWPORT,
                scale: f32::from(scale),
            },
            scene,
            Color::rgba(0.0, 0.0, 0.0, 1.0),
        )
        .expect("render scaled layer");
}

const fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * VIEWPORT_WIDTH + x) * 4) as usize;
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}
