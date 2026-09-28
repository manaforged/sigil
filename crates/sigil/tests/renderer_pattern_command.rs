#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "scene_support/mod.rs"]
mod scene_support;

use scene_support::color;
use sigil::hal::{Color, Point, Rect};
use sigil::scene::{
    CommandScene, CornerRadii, PaintFill, PatternKind, SceneBuilder, SceneCommand,
    ScenePatternStyle, Stroke, StrokeAlign, StrokeStyle,
};
use sigil::style::Rgba8;
use sigil_kittest::gpu::{Capture, render_capture};

const VIEWPORT: (f32, f32) = (220.0, 80.0);
const VIEWPORT_WIDTH: u32 = 220;
const VIEWPORT_HEIGHT: u32 = 80;
const CAPTURE: Capture = Capture {
    viewport: VIEWPORT,
    size_px: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
    scale: 1.0,
    clear: Color::rgba(0.05, 0.06, 0.09, 1.0),
};

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn pattern_commands_walk_and_populate_fill_pipeline_cache() {
    let scene = pattern_scene();
    assert_pattern_command_shape(&scene);
    assert!(matches!(PaintFill::Pattern(slash()), PaintFill::Pattern(_)));

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
