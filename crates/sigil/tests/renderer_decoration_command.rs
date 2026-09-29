#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "scene_support/mod.rs"]
mod scene_support;

use scene_support::{color, rect};
use sigil::geometry::Rect;
use sigil::scene::{
    CommandScene, DecorationKind, SceneBuilder, SceneCommand, SceneDecorationStyle,
};
use sigil::style::{Color, Rgba8};
use sigil_kittest::gpu::{Capture, render_capture};

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
fn decoration_commands_walk_and_populate_decoration_pipeline_cache() {
    let scene = decoration_scene();
    assert_decoration_command_shape(&scene);

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
    render_capture(
        &mut renderer,
        (&ready_device, &ready_queue),
        &scene,
        &CAPTURE,
    )
    .expect("render_capture");

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

fn approx_eq(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON
}
