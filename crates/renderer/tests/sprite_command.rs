//! Sprite command integration coverage.

#![allow(missing_docs)]
#![allow(clippy::panic)]

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
    BUILTIN_SPRITE_ATLAS_ID, CommandScene, OffscreenTarget, SceneBuilder, SceneCommand, SpriteKind,
    SpriteSource, WgpuRenderer,
};
use sigil_style::Rgba8;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (220.0, 80.0);
const VIEWPORT_WIDTH: u32 = 220;
const VIEWPORT_HEIGHT: u32 = 80;

#[test]
fn sprite_commands_walk_and_populate_sprite_pipeline_cache() {
    let scene = sprite_scene();
    assert_sprite_command_shape(&scene);

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device - skipping sprite command walker test: {err}");
        return;
    }

    let ready_device = renderer.device().expect("headless device").clone();
    let ready_queue = renderer.queue().expect("headless queue").clone();
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);

    let (_, _, _, _, sprite_pipelines) = renderer
        .pipeline_cache_counts()
        .expect("renderer should have headless cache state");
    assert!(sprite_pipelines > 0);
}

fn assert_sprite_command_shape(scene: &CommandScene) {
    let [
        SceneCommand::Sprite(mono),
        SceneCommand::Sprite(subpixel),
        SceneCommand::Sprite(poly),
    ] = scene.commands.as_slice()
    else {
        panic!("expected three sprite draw commands");
    };

    assert_eq!(mono.kind, SpriteKind::Mono);
    assert_eq!(subpixel.kind, SpriteKind::Subpixel);
    assert_eq!(poly.kind, SpriteKind::Poly);
    assert_eq!(mono.source.atlas, BUILTIN_SPRITE_ATLAS_ID);
    assert_eq!(subpixel.source.atlas, BUILTIN_SPRITE_ATLAS_ID);
    assert_eq!(poly.source.atlas, BUILTIN_SPRITE_ATLAS_ID);
}

fn sprite_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.draw_sprite(
        source(0.0, 0.0, 0.0),
        rect(0.0),
        SpriteKind::Mono,
        color(220, 60, 60, 255),
    );
    builder.draw_sprite(
        source(200.0, 0.0, 0.35),
        rect(56.0),
        SpriteKind::Subpixel,
        color(28, 32, 44, 255),
    );
    builder.draw_sprite(
        source(400.0, 0.0, 0.0),
        rect(112.0),
        SpriteKind::Poly,
        color(80, 160, 240, 255),
    );
    builder.finish()
}

fn render_scene(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sprite-command capture"),
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

const fn source(x: f32, y: f32, phase: f32) -> SpriteSource {
    SpriteSource {
        atlas: BUILTIN_SPRITE_ATLAS_ID,
        region: Rect::new(Point::new(x, y), 200.0, 200.0),
        phase,
    }
}

const fn rect(x: f32) -> Rect {
    Rect::new(Point::new(x, 0.0), 48.0, 48.0)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
