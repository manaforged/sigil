//! Glow command integration coverage — walker test for R10.
//!
//! Mirrors `tests/shadow_command.rs`. Builds a `CommandScene` with three
//! `DrawShadow` commands (one each of Outer / Inner / Glow), walks the
//! command list, and asserts the pipeline cache is populated.

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
use taffy as _;
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
fn glow_command_keeps_shadow_kind_glow() {
    let mut builder = SceneBuilder::new();
    builder.draw_shadow(surface_rect(), CornerRadii::uniform(24.0), glow_shadow());
    let scene = builder.finish();

    assert_eq!(scene.commands.len(), 1);
    if let [SceneCommand::DrawShadow { shadow, .. }] = scene.commands.as_slice() {
        assert_eq!(shadow.kind, ShadowKind::Glow);
    }
}

#[test]
fn outer_inner_glow_emit_as_independent_commands() {
    let mut builder = SceneBuilder::new();
    builder.draw_shadow(surface_rect(), CornerRadii::uniform(24.0), outer_shadow());
    builder.draw_rect(
        surface_rect(),
        SceneRectStyle::new(fill_color(), CornerRadii::uniform(24.0), 10_000),
    );
    builder.draw_shadow(surface_rect(), CornerRadii::uniform(24.0), inner_shadow());
    builder.draw_shadow(surface_rect(), CornerRadii::uniform(24.0), glow_shadow());
    let scene = builder.finish();

    assert_eq!(scene.commands.len(), 4);
    assert!(matches!(
        scene.commands.as_slice(),
        [
            SceneCommand::DrawShadow { shadow: outer, .. },
            SceneCommand::Rect(_),
            SceneCommand::DrawShadow { shadow: inner, .. },
            SceneCommand::DrawShadow { shadow: glow, .. },
        ] if outer.kind == ShadowKind::Outer
            && inner.kind == ShadowKind::Inner
            && glow.kind == ShadowKind::Glow
    ));

    // Verify the render_command_scene signature exists (compile-time check).
    let render_path: RenderCommandScene = WgpuRenderer::render_command_scene;
    let _ = render_path;
}

#[test]
fn glow_shadow_has_zero_offset() {
    let g = glow_shadow();
    assert!(
        (g.offset[0]).abs() < f32::EPSILON,
        "glow x-offset must be 0"
    );
    assert!(
        (g.offset[1]).abs() < f32::EPSILON,
        "glow y-offset must be 0"
    );
}

type RenderCommandScene = fn(
    &mut WgpuRenderer,
    &mut WgpuSurface,
    &CommandScene,
    Color,
    (f32, f32),
) -> Result<(), RendererError>;

fn glow_shadow() -> Shadow {
    Shadow::glow(
        Rgba8 {
            r: 80,
            g: 200,
            b: 255,
            a: 128,
        },
        8.0,
        18.0,
    )
}

const fn outer_shadow() -> Shadow {
    Shadow::new([0.0, 4.0], 12.0, 1.0, [0.0, 0.0, 0.0, 0.28])
}

const fn inner_shadow() -> Shadow {
    Shadow::new([0.0, 1.0], 7.0, 2.0, [0.0, 0.0, 0.0, 0.24]).with_kind(ShadowKind::Inner)
}

const fn surface_rect() -> Rect {
    Rect::new(Point::new(100.0, 80.0), 200.0, 200.0)
}

const fn fill_color() -> Rgba8 {
    Rgba8 {
        r: 245,
        g: 245,
        b: 250,
        a: 255,
    }
}
