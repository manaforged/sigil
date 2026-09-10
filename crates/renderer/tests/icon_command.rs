#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, OffscreenTarget, SceneBuilder, SceneCommand, SpriteKind, SpriteSource,
    WgpuRenderer, scenes::BUILTIN_ICON_ATLAS_BASE_ID,
};
use sigil_style::Rgba8;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (360.0, 80.0);
const VIEWPORT_WIDTH: u32 = 360;
const VIEWPORT_HEIGHT: u32 = 80;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn icon_commands_walk_and_populate_sprite_pipeline_cache() {
    let scene = icon_scene();
    assert_icon_command_shape(&scene);

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device - skipping icon command walker test: {err}");
        return;
    }

    let ready_device = renderer.device().expect("headless device").clone();
    let ready_queue = renderer.queue().expect("headless queue").clone();
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);

    let (_, _, _, _, sprite_pipelines) = renderer
        .pipeline_cache_counts()
        .expect("renderer should have headless cache state");
    assert!(sprite_pipelines > 0);
}

fn assert_icon_command_shape(scene: &CommandScene) {
    let [
        SceneCommand::Sprite(heart),
        SceneCommand::Sprite(gear),
        SceneCommand::Sprite(search),
        SceneCommand::Sprite(arrow),
        SceneCommand::Sprite(close),
    ] = scene.commands.as_slice()
    else {
        panic!("expected five icon draw commands");
    };

    for (index, draw) in [heart, gear, search, arrow, close].into_iter().enumerate() {
        assert_eq!(draw.kind, SpriteKind::Mono);
        assert_eq!(draw.source.atlas, BUILTIN_ICON_ATLAS_BASE_ID + index as u32);
    }
}

fn icon_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    for index in 0..5 {
        builder.draw_sprite(
            source(BUILTIN_ICON_ATLAS_BASE_ID + index, 0.0, 0.0),
            rect(index as f32 * 64.0),
            SpriteKind::Mono,
            color(220, 220, 240, 255),
        );
    }
    builder.finish()
}

fn render_scene(
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &CommandScene,
) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("icon-command capture"),
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

const fn source(atlas: u32, x: f32, y: f32) -> SpriteSource {
    SpriteSource {
        atlas,
        region: Rect::new(Point::new(x, y), 128.0, 128.0),
        phase: 0.0,
    }
}

const fn rect(x: f32) -> Rect {
    Rect::new(Point::new(x, 0.0), 48.0, 48.0)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
