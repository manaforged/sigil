#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

#[path = "scene_support/mod.rs"]
mod scene_support;

use scene_support::color;
use sigil::geometry::{Point, Rect};
use sigil::renderer::scenes::BUILTIN_ICON_ATLAS_BASE_ID;
use sigil::scene::{CommandScene, SceneBuilder, SceneCommand, SpriteKind, SpriteSource};
use sigil::style::Color;
use sigil_kittest::gpu::{Capture, render_capture};

const VIEWPORT: (f32, f32) = (360.0, 80.0);
const VIEWPORT_WIDTH: u32 = 360;
const VIEWPORT_HEIGHT: u32 = 80;
const CAPTURE: Capture = Capture {
    viewport: VIEWPORT,
    size_px: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT],
    scale: 1.0,
    clear: Color::rgba(0.05, 0.06, 0.09, 1.0),
};

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn icon_commands_walk_and_populate_sprite_pipeline_cache() {
    let scene = icon_scene();
    assert_icon_command_shape(&scene);

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
