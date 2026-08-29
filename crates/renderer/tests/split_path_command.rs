#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Split-path command walker integration coverage.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, CornerRadii, SceneBuilder, SceneCommand, SceneRectStyle, ScreenSubmission,
    WgpuRenderer,
};
use sigil_style::Rgba8;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (220.0, 80.0);
const VIEWPORT_WIDTH: u32 = 220;
const VIEWPORT_HEIGHT: u32 = 80;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn submit_screen_walks_basic_command_scene() {
    let scene = basic_scene();
    assert_basic_command_shape(&scene);

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device - skipping split path command test: {err}");
        return;
    }

    let ready_device = renderer.device().expect("headless device").clone();
    let ready_queue = renderer.queue().expect("headless queue").clone();
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);

    let (scene_pipelines, _, _, _, _) = renderer
        .pipeline_cache_counts()
        .expect("renderer should have headless cache state");
    assert!(scene_pipelines > 0);
}

fn assert_basic_command_shape(scene: &CommandScene) {
    let [SceneCommand::Rect(draw)] = scene.commands.as_slice() else {
        panic!("expected one rect draw command");
    };
    assert_eq!(draw.rect, rect(24.0, 16.0, 172.0, 48.0));
}

fn basic_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.draw_rect(
        rect(24.0, 16.0, 172.0, 48.0),
        SceneRectStyle::new(color(220, 60, 60, 255), CornerRadii::uniform(8.0), 10_000),
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
        label: Some("split-path-command capture"),
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
        .submit_screen(
            &ScreenSubmission {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: VIEWPORT,
                scale: 1.0,
                clear: Some(Color::rgba(0.05, 0.06, 0.09, 1.0)),
            },
            scene,
        )
        .expect("submit_screen");
}

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
