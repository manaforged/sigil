//! Decoration command integration coverage.

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
use tracing as _;
#[cfg(windows)]
use windows_core as _;

use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, DecorationKind, OffscreenTarget, SceneBuilder, SceneCommand,
    SceneDecorationStyle, WgpuRenderer,
};
use sigil_style::Rgba8;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (220.0, 120.0);
const VIEWPORT_WIDTH: u32 = 220;
const VIEWPORT_HEIGHT: u32 = 120;

#[test]
fn decoration_commands_walk_and_populate_decoration_pipeline_cache() {
    let scene = decoration_scene();
    assert_decoration_command_shape(&scene);

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device - skipping decoration command walker test: {err}");
        return;
    }

    let ready_device = renderer.device().expect("headless device").clone();
    let ready_queue = renderer.queue().expect("headless queue").clone();
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);

    let stats = renderer.last_frame_stats();
    assert_eq!(stats.scene_commands, 3);
    assert_eq!(stats.decoration_batches, 1);
    assert_eq!(stats.decoration_instances, 3);

    let decoration_pipelines = renderer
        .decoration_pipeline_cache_count()
        .expect("renderer should have headless cache state");
    assert!(decoration_pipelines > 0);
}

fn assert_decoration_command_shape(scene: &CommandScene) {
    let [
        SceneCommand::Decoration(underline),
        SceneCommand::Decoration(strikethrough),
        SceneCommand::Decoration(wavy),
    ] = scene.commands.as_slice()
    else {
        panic!("expected three decoration draw commands");
    };

    assert_eq!(underline.kind, DecorationKind::Underline);
    assert_eq!(strikethrough.kind, DecorationKind::Strikethrough);
    assert_eq!(wavy.kind, DecorationKind::WavyUnderline);
    assert!(approx_eq(underline.line_width, 2.0));
    assert!(approx_eq(strikethrough.period, 0.0));
    assert!(approx_eq(wavy.period, 32.0));
}

fn decoration_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.draw_decoration(decoration_style(
        DecorationKind::Underline,
        rect(8.0, 16.0, 160.0, 6.0),
        color(100, 200, 255, 255),
        2.0,
        0.0,
    ));
    builder.draw_decoration(decoration_style(
        DecorationKind::Strikethrough,
        rect(8.0, 48.0, 160.0, 4.0),
        color(220, 60, 60, 255),
        2.0,
        0.0,
    ));
    builder.draw_decoration(decoration_style(
        DecorationKind::WavyUnderline,
        rect(8.0, 80.0, 160.0, 16.0),
        color(220, 140, 60, 255),
        2.5,
        32.0,
    ));
    builder.finish()
}

fn render_scene(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("decoration-command capture"),
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
                scale: 1.0,
            },
            scene,
            Color::rgba(0.05, 0.06, 0.09, 1.0),
        )
        .expect("render_command_scene_offscreen");
}

const fn decoration_style(
    kind: DecorationKind,
    band: Rect,
    color: Rgba8,
    line_width: f32,
    period: f32,
) -> SceneDecorationStyle {
    SceneDecorationStyle {
        kind,
        band,
        color,
        opacity: 10_000,
        period,
        line_width,
    }
}

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn approx_eq(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON
}
