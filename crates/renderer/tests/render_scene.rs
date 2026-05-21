//! `WgpuRenderer::render_scene` headless tests.
//!
//! The shader-validation regression that crashed `hello-style` on launch
//! (a WGSL reserved-keyword collision in `RectInstance`) had no test
//! coverage. These tests construct the scene pipeline against a real GPU
//! adapter, which forces shader validation, and exercise the public
//! method end-to-end against a headless render target.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use hal as _;
use half as _;
use icons as _;
use image as _;
use kernel as _;
use layout as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use shell as _;
use skrifa as _;
use smallvec as _;
use style as _;
use text as _;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use hal::{Color, Point, Rect};
use renderer::{
    CommandScene, CornerRadii, RendererError, Scene, SceneBuilder, SceneCommand, SceneRectStyle,
    WgpuRenderer, WgpuSurface,
};
use style::Rgba8;

#[test]
fn renderer_construction_does_not_panic() {
    // Cheap construction — only allocates the wgpu Instance.
    // Catches API-shape regressions in WgpuRenderer::new without
    // requiring a window.
    let _ = WgpuRenderer::new().expect("renderer construction");
}

#[test]
fn renderer_new_repeatable() {
    // Two construction sites must not contend on global wgpu state.
    let _a = WgpuRenderer::new().expect("first renderer");
    let _b = WgpuRenderer::new().expect("second renderer");
}

#[test]
fn renderer_pre_surface_accessors_return_none() {
    // Bulletproof contract: device/queue accessors must not panic
    // before create_surface (which is when launch first calls them
    // for the lazy surface init path).
    let r = WgpuRenderer::new().expect("renderer");
    assert!(r.device().is_none());
    assert!(r.queue().is_none());
}

#[test]
fn render_scene_signature_keeps_logical_viewport_tuple() {
    type RenderScene = fn(
        &mut WgpuRenderer,
        &mut WgpuSurface,
        &Scene,
        Color,
        (f32, f32),
    ) -> Result<(), RendererError>;

    let method: RenderScene = WgpuRenderer::render_scene;
    let _ = method;
}

#[test]
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
fn render_command_scene_paints_save_layer_with_opacity() {
    let mut builder = SceneBuilder::new();
    builder.with_layer(0.5, |scene| {
        scene.draw_rect(rect(0.0, 0.0, 10.0, 10.0), rect_style());
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
fn render_command_scene_transform_affects_descendants() {
    let transform = [[1.0, 0.0, 8.0], [0.0, 1.0, 12.0]];
    let mut builder = SceneBuilder::new();
    builder.with_transform(transform, |scene| {
        scene.draw_rect(rect(1.0, 2.0, 10.0, 10.0), rect_style());
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
    Color,
    (f32, f32),
) -> Result<(), RendererError>;

fn nested_clip_scene() -> CommandScene {
    let mut builder = SceneBuilder::new();
    builder.with_clip(rect(0.0, 0.0, 100.0, 100.0), |scene| {
        scene.with_clip(rect(10.0, 10.0, 40.0, 40.0), |scene| {
            scene.draw_rect(rect(5.0, 5.0, 80.0, 80.0), rect_style());
        });
    });
    builder.finish()
}

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

const fn rect_style() -> SceneRectStyle {
    SceneRectStyle::new(
        Rgba8 {
            r: 180,
            g: 20,
            b: 80,
            a: 255,
        },
        CornerRadii::ZERO,
        10_000,
    )
}
