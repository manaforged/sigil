//! Inner-shadow command integration coverage.

#![allow(missing_docs)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal::{Color, Point, Rect};
use sigil_kernel as _;
use sigil_layout as _;
use sigil_shell as _;
use sigil_style::Rgba8;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use sigil_icons as _;

use sigil_renderer::{
    CommandScene, CornerRadii, RendererError, SceneBuilder, SceneCommand, SceneRectStyle, Shadow,
    ShadowKind, WgpuRenderer, WgpuSurface,
};

#[test]
fn inner_shadow_command_keeps_shadow_kind() {
    let mut builder = SceneBuilder::new();
    builder.draw_shadow(rect(), CornerRadii::uniform(10.0), inner_shadow());
    let scene = builder.finish();

    match scene.commands.as_slice() {
        [SceneCommand::DrawShadow { shadow, .. }] => assert_eq!(shadow.kind, ShadowKind::Inner),
        other => assert_eq!(other.len(), 1),
    }
}

#[test]
fn outer_and_inner_shadows_emit_as_independent_commands() {
    let mut builder = SceneBuilder::new();
    builder.draw_shadow(rect(), CornerRadii::uniform(10.0), outer_shadow());
    builder.draw_rect(
        rect(),
        SceneRectStyle::new(
            color(240, 245, 250, 255),
            CornerRadii::uniform(10.0),
            10_000,
        ),
    );
    builder.draw_shadow(rect(), CornerRadii::uniform(10.0), inner_shadow());
    let scene = builder.finish();

    assert!(matches!(
        scene.commands.as_slice(),
        [
            SceneCommand::DrawShadow { shadow: outer, .. },
            SceneCommand::Rect(_),
            SceneCommand::DrawShadow { shadow: inner, .. },
        ] if outer.kind == ShadowKind::Outer && inner.kind == ShadowKind::Inner
    ));

    let render_path: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = render_path;
}

type RenderCommandScene = fn(
    &mut WgpuRenderer,
    &mut WgpuSurface,
    &CommandScene,
    Color,
    (f32, f32),
) -> Result<(), RendererError>;

const fn outer_shadow() -> Shadow {
    Shadow::new([0.0, 3.0], 8.0, 0.0, [0.0, 0.0, 0.0, 0.18])
}

const fn inner_shadow() -> Shadow {
    Shadow::new([0.0, 1.0], 7.0, 2.0, [0.0, 0.0, 0.0, 0.24]).with_kind(ShadowKind::Inner)
}

const fn rect() -> Rect {
    Rect::new(Point::new(16.0, 12.0), 96.0, 40.0)
}

const fn color(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
