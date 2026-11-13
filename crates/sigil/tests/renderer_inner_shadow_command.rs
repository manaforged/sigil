#![cfg(feature = "kittest")]

use crate::scene_support;

use scene_support::color;
use sigil::geometry::{Point, Rect};
use sigil::renderer::{WgpuRenderer, WgpuSurface};
use sigil::scene::{CommandScene, DrawRect, SceneBuilder, SceneCommand, Shadow, ShadowKind};
use sigil::style::Rgba;

#[test]
fn inner_shadow_command_keeps_shadow_kind() {
    let mut builder = SceneBuilder::new();
    builder.draw_shadow(rect(), [10.0; 4], inner_shadow());
    let scene = builder.finish();

    match scene.commands() {
        [SceneCommand::DrawShadow { shadow, .. }] => {
            assert_eq!(shadow.shadow_kind(), ShadowKind::Inner);
        }
        other => assert_eq!(other.len(), 1),
    }
}

#[test]
fn outer_and_inner_shadows_emit_as_independent_commands() {
    let mut builder = SceneBuilder::new();
    builder.draw_shadow(rect(), [10.0; 4], outer_shadow());
    builder.draw_rect(DrawRect::new(rect(), color(240, 245, 250, 255)).radius(10.0));
    builder.draw_shadow(rect(), [10.0; 4], inner_shadow());
    let scene = builder.finish();

    assert!(matches!(
        scene.commands(),
        [
            SceneCommand::DrawShadow { shadow: outer, .. },
            SceneCommand::Rect(_),
            SceneCommand::DrawShadow { shadow: inner, .. },
        ] if outer.shadow_kind() == ShadowKind::Outer && inner.shadow_kind() == ShadowKind::Inner
    ));

    let render_path: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = render_path;
}

type RenderCommandScene = fn(
    &mut WgpuRenderer,
    &mut WgpuSurface,
    &CommandScene,
    Rgba,
    (f32, f32),
) -> Result<(), sigil::Error>;

const fn outer_shadow() -> Shadow {
    Shadow::new([0.0, 3.0], 8.0, 0.0, [0.0, 0.0, 0.0, 0.18])
}

const fn inner_shadow() -> Shadow {
    Shadow::new([0.0, 1.0], 7.0, 2.0, [0.0, 0.0, 0.0, 0.24]).kind(ShadowKind::Inner)
}

const fn rect() -> Rect {
    Rect::new(Point::new(16.0, 12.0), 96.0, 40.0)
}
