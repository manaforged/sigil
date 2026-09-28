#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic)]

#[path = "scene_support/mod.rs"]
mod scene_support;

use scene_support::color;
use sigil::hal::{Color, Point, Rect};
use sigil::renderer::BUILTIN_SPRITE_ATLAS_ID;
use sigil::scene::{CommandScene, SceneBuilder, SceneCommand, SpriteKind, SpriteSource};
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
fn sprite_commands_walk_and_populate_sprite_pipeline_cache() {
    let scene = sprite_scene();
    assert_sprite_command_shape(&scene);

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
