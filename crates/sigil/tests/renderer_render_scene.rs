#![cfg(feature = "kittest")]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "scene_support/mod.rs"]
mod scene_support;

use scene_support::rect;
use sigil::renderer::{WgpuRenderer, WgpuSurface};
use sigil::scene::{CommandScene, DrawRect, SceneBuilder, SceneCommand};
use sigil::style::{Color, Rgba};

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn renderer_construction_does_not_panic() {
    let _ = WgpuRenderer::new();
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn renderer_new_repeatable() {
    let _a = WgpuRenderer::new();
    let _b = WgpuRenderer::new();
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn renderer_pre_surface_accessors_return_none() {
    let r = WgpuRenderer::new();
    assert!(r.device().is_none());
    assert!(r.queue().is_none());
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn render_command_scene_paints_nested_clip() {
    let scene = nested_clip_scene();

    assert!(matches!(
        scene.commands.as_slice(),
        [
            SceneCommand::PushClip(_),
            SceneCommand::PushClip(_),
            SceneCommand::Rect(_),
            SceneCommand::PopClip,
            SceneCommand::PopClip,
        ]
    ));
    let method: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = method;
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn render_command_scene_paints_save_layer_with_opacity() {
    let mut builder = SceneBuilder::new();
    builder.with_layer(0.5, |scene| {
        scene.draw_rect(rect_style(rect(0.0, 0.0, 10.0, 10.0)));
    });
    let scene = builder.finish();

    assert!(matches!(
        scene.commands.as_slice(),
        [
            SceneCommand::SaveLayer { opacity },
            SceneCommand::Rect(_),
            SceneCommand::RestoreLayer,
        ] if (*opacity - 0.5).abs() < f32::EPSILON
    ));
    let method: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = method;
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn render_command_scene_transform_affects_descendants() {
    let transform = [[1.0, 0.0, 8.0], [0.0, 1.0, 12.0]];
    let mut builder = SceneBuilder::new();
    builder.with_transform(transform, |scene| {
        scene.draw_rect(rect_style(rect(1.0, 2.0, 10.0, 10.0)));
    });
    let scene = builder.finish();

    assert!(matches!(
        scene.commands.as_slice(),
        [
            SceneCommand::PushTransform(t),
            SceneCommand::Rect(_),
            SceneCommand::PopTransform,
        ] if *t == transform
    ));
    let method: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = method;
}

type RenderCommandScene = fn(
    &mut WgpuRenderer,
    &mut WgpuSurface,
    &CommandScene,
    Rgba,
    (f32, f32),
) -> Result<(), sigil::Error>;

fn nested_clip_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.with_clip(rect(0.0, 0.0, 100.0, 100.0), |scene| {
        scene.with_clip(rect(10.0, 10.0, 40.0, 40.0), |scene| {
            scene.draw_rect(rect_style(rect(5.0, 5.0, 80.0, 80.0)));
        });
    });
    builder.finish()
}

const fn rect_style(rect: sigil::geometry::Rect) -> DrawRect {
    DrawRect::new(
        rect,
        Color {
            r: 180,
            g: 20,
            b: 80,
            a: 255,
        },
    )
}
