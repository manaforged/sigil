//! Flat-scene to command-scene adapter tests.

#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used)]
#![allow(clippy::redundant_pub_crate)]

use ahash as _;
use bytemuck as _;
use criterion as _;
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_shell as _;
use sigil_text as _;
use skrifa as _;
use smallvec as _;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use sigil_icons as _;

use sigil_hal::Size;
use sigil_kernel::{Cx, NodeId, TextContent, prelude::*};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use sigil_renderer::{Scene, SceneCommand, SceneItem, emit_scene};
use sigil_style::NodeStyleExt;

mod scene {
    pub(super) use sigil_renderer::{Scene, SceneItem, shape_glyph_run};
}

mod scene_builder {
    pub(super) use sigil_renderer::SceneBuilder;
}

mod scene_command {
    use sigil_renderer::{CommandScene, SceneRectStyle};

    mod adapter {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/scene_command/adapter.rs"
        ));
    }

    pub(super) use adapter::flat_to_commands;
}

#[test]
fn adapter_preserves_rect_count() {
    let scene = fixture_scene();
    let commands = scene_command::flat_to_commands(&scene);

    assert_eq!(rect_count(&scene), draw_rect_count(&commands.commands));
}

#[test]
fn adapter_preserves_text_content_for_interim_path() {
    let scene = fixture_scene();
    let commands = scene_command::flat_to_commands(&scene);

    assert!(
        commands.commands.iter().any(|command| matches!(
            command,
            SceneCommand::GlyphRun(draw) if draw.run.text == "Adapter text"
                && draw.run.has_positioned_glyphs()
        )),
        "adapter must preserve text and carry shaped glyph positions"
    );
}

#[test]
fn adapter_produces_no_push_pop_for_flat_input() {
    let scene = fixture_scene();
    let commands = scene_command::flat_to_commands(&scene);

    assert!(commands.commands.iter().all(|command| {
        matches!(
            command,
            SceneCommand::Rect(_) | SceneCommand::Gradient(_) | SceneCommand::GlyphRun(_)
        )
    }));
}

#[test]
fn adapter_round_trip_matches_original_emit_scene() {
    let scene = fixture_scene();
    let commands = scene_command::flat_to_commands(&scene);

    assert_eq!(scene.items().len(), commands.commands.len());
    for (item, command) in scene.items().iter().zip(commands.commands) {
        match (item, command) {
            (SceneItem::Rect(rect), SceneCommand::Rect(draw)) => {
                assert_eq!(rect.rect, draw.rect);
                assert_eq!(rect.color, draw.color);
                assert_eq!(rect.border_radius, draw.border_radius);
                assert_eq!(rect.opacity, draw.opacity);
            }
            (SceneItem::Text(text), SceneCommand::GlyphRun(draw)) => {
                assert_eq!(text.rect, draw.run.rect);
                assert_eq!(text.text, draw.run.text);
                assert_eq!(text.color, draw.run.color);
                assert_eq!(text.style, draw.run.style);
                assert_eq!(text.opacity, draw.run.opacity);
            }
            other => panic!("adapter changed item kind: {other:?}"),
        }
    }
}

fn fixture_scene() -> Scene {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(160.0, 80.0))
            .class("bg-surface")
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(160.0, 24.0))
                    .class("bg-accent"),
            )
            .child(text("Adapter text")),
    );
    lay_out(&mut cx, root);
    #[allow(deprecated)]
    let scene = emit_scene(&cx, root).to_flat();
    scene
}

fn lay_out(cx: &mut Cx, root: NodeId) {
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
        .compute(cx, root, Size::fixed(160.0, 80.0))
        .expect("layout computes");
}

fn rect_count(scene: &Scene) -> usize {
    scene
        .items()
        .iter()
        .filter(|item| matches!(item, SceneItem::Rect(_)))
        .count()
}

fn draw_rect_count(commands: &[SceneCommand]) -> usize {
    commands
        .iter()
        .filter(|command| matches!(command, SceneCommand::Rect(_)))
        .count()
}

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(96.0, 18.0)
    }
}
