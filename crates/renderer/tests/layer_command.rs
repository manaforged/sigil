//! Layer command integration coverage.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal as _;
use sigil_icons as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use sigil_style as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use taffy as _;
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, CornerRadii, LayerEffect, LayerScope, OffscreenTarget, SceneBuilder,
    SceneCommand, SceneRectStyle, WgpuRenderer,
};
use sigil_style::Rgba8;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (220.0, 120.0);
const VIEWPORT_WIDTH: u32 = 220;
const VIEWPORT_HEIGHT: u32 = 120;

#[test]
fn layer_commands_walk_and_initialize_layer_pool() {
    let scene = layer_scene();
    assert_layer_command_shape(&scene);

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device - skipping layer command walker test: {err}");
        return;
    }

    let ready_device = renderer.device().expect("headless device").clone();
    let ready_queue = renderer.queue().expect("headless queue").clone();
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);

    let (scene_pipelines, _, _, _, _) = renderer
        .pipeline_cache_counts()
        .expect("renderer should have headless cache state");
    assert!(scene_pipelines > 0);
    assert!(renderer.layer_pool_ready_for_tests());
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

fn render_scene(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("layer-command capture"),
        size: wgpu::Extent3d {
            width: VIEWPORT_WIDTH,
            height: VIEWPORT_HEIGHT,
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
            },
            scene,
            Color::rgba(0.05, 0.06, 0.09, 1.0),
        )
        .expect("render_command_scene_offscreen");
}

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
