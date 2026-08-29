#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Pattern command integration coverage.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil_hal::{Color, Point, Rect};
use sigil_renderer::{
    CommandScene, CornerRadii, Fill, OffscreenTarget, PatternKind, SceneBuilder, SceneCommand,
    ScenePatternStyle, Stroke, StrokeAlign, StrokeStyle, WgpuRenderer,
};
use sigil_style::Rgba8;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEWPORT: (f32, f32) = (220.0, 80.0);
const VIEWPORT_WIDTH: u32 = 220;
const VIEWPORT_HEIGHT: u32 = 80;

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn pattern_commands_walk_and_populate_fill_pipeline_cache() {
    let scene = pattern_scene();
    assert_pattern_command_shape(&scene);
    assert!(matches!(Fill::Pattern(slash()), Fill::Pattern(_)));

    let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
    if let Err(err) = renderer.ensure_headless_for_tests() {
        eprintln!("no headless renderer device - skipping pattern command walker test: {err}");
        return;
    }

    let ready_device = renderer.device().expect("headless device").clone();
    let ready_queue = renderer.queue().expect("headless queue").clone();
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);
    render_scene(&mut renderer, &ready_device, &ready_queue, &scene);

    let stats = renderer.last_frame_stats();
    assert_eq!(stats.scene_commands, 3);
    assert_eq!(stats.fill_batches, 1);
    assert_eq!(stats.fill_instances, 3);

    let (_, fill_pipelines, _, _, _) = renderer
        .pipeline_cache_counts()
        .expect("renderer should have headless cache state");
    assert!(fill_pipelines > 0);
}

fn assert_pattern_command_shape(scene: &CommandScene) {
    let [
        SceneCommand::Pattern(slash_draw),
        SceneCommand::Pattern(checker_draw),
        SceneCommand::Pattern(stroked_draw),
    ] = scene.commands.as_slice()
    else {
        panic!("expected three pattern draw commands");
    };

    assert_eq!(slash_draw.pattern, slash());
    assert!(matches!(checker_draw.pattern, PatternKind::Checker { .. }));
    assert_eq!(stroked_draw.pattern, slash());
    assert_eq!(stroked_draw.stroke, Some(outside_stroke()));
}

fn pattern_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.draw_pattern(
        rect(0.0, 0.0),
        ScenePatternStyle::new(slash(), CornerRadii::uniform(8.0), 10_000),
    );
    builder.draw_pattern(
        rect(56.0, 0.0),
        ScenePatternStyle::new(checker(), CornerRadii::uniform(8.0), 10_000),
    );
    builder.draw_pattern(
        rect(112.0, 0.0),
        ScenePatternStyle::new(slash(), CornerRadii::uniform(8.0), 10_000)
            .with_stroke(outside_stroke()),
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
        label: Some("pattern-command capture"),
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

const fn slash() -> PatternKind {
    PatternKind::Slash {
        color: color(255, 255, 255, 255),
        period: 40.0,
        line_width: 16.0,
    }
}

const fn checker() -> PatternKind {
    PatternKind::Checker {
        color_a: color(220, 60, 60, 255),
        color_b: color(245, 245, 250, 255),
        cell: 40.0,
    }
}

const fn outside_stroke() -> Stroke {
    Stroke {
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        width: 4.0,
        style: StrokeStyle::Solid,
        align: StrokeAlign::Outside,
    }
}

const fn rect(x: f32, y: f32) -> Rect {
    Rect::new(Point::new(x, y), 48.0, 48.0)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
