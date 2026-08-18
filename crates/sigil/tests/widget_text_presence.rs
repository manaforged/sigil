//! Widgets that display text must actually emit it.
//!
//! On 2026-07-20 a dropdown rendered with its label gone entirely. Nothing
//! caught it: the suite was green, and re-baking the golden simply wrote a
//! picture of the bug. Pixel budgets cannot see it either — the missing
//! label was 146 of 3494 non-background pixels, ~4% of the image, far under
//! any threshold that would not also fire on ordinary re-bakes.
//!
//! Scope, learned by testing the guard against the real bug: scene
//! presence alone is NOT enough. `emit_scene` emits a glyph run for a text
//! node even when its container resolved to zero width, so the run is in
//! the scene while nothing is visible. These tests therefore also require
//! the run to have a non-degenerate box. The collapse itself is caught
//! independently by `layout_invariants`; this file guards the text-specific
//! half (a widget that silently stops emitting its content at some width).

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used, clippy::panic)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_renderer::{SceneCommand, emit_scene};

struct ProportionalText;
impl TextMeasure for ProportionalText {
    fn measure(&self, content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        let len = match content {
            TextContent::Static(s) => s.len(),
            TextContent::Reactive(sig) => sig.peek().len(),
            TextContent::Compute(build) => build().len(),
        };
        #[allow(clippy::cast_precision_loss)]
        sigil_hal::LogicalSize::new(len as f32 * 7.0, 16.0)
    }
}

/// Every string the scene paints with a non-degenerate box, at `width`.
///
/// Runs whose own extent is zero are excluded deliberately: a glyph run
/// inside a collapsed container is emitted but paints nothing, and counting
/// it would make this guard agree that an invisible label is present.
fn painted_text(width: f32, build: impl FnOnce() -> NodeBuilder) -> Vec<String> {
    let mut found = Vec::new();
    sigil::reactive::Runtime::scope("text-presence", || {
        let mut cx = Cx::new_root();
        let root = cx.add(build());
        let mut layout = LayoutEngine::with_text_measure(Box::new(ProportionalText));
        layout
            .compute(&mut cx, root, Size::fixed(width, 400.0))
            .expect("layout computes");
        found = emit_scene(&cx, root)
            .commands
            .iter()
            .filter_map(|c| match c {
                SceneCommand::GlyphRun(g) if g.run.rect.width > 0.5 && g.run.rect.height > 0.5 => {
                    Some(g.run.text.clone())
                }
                _ => None,
            })
            .collect();
    });
    found
}

fn assert_paints(width: f32, needle: &str, build: impl FnOnce() -> NodeBuilder) {
    let painted = painted_text(width, build);
    assert!(
        painted.iter().any(|t| t.contains(needle)),
        "at width {width}: expected some glyph run containing {needle:?}, \
         but the scene painted {painted:?}",
    );
}

/// Narrow enough to squeeze the widget, wide enough to be roomy.
const WIDTHS: [f32; 3] = [140.0, 320.0, 900.0];

#[test]
fn dropdown_paints_its_selected_option() {
    for width in WIDTHS {
        assert_paints(width, "Banana", || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(dropdown(
                    signal(0_usize),
                    vec!["Banana".into(), "Cherry".into()],
                ))
        });
    }
}

#[test]
fn button_paints_its_label() {
    for width in WIDTHS {
        assert_paints(width, "Save", || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(button("Save"))
        });
    }
}

#[test]
fn text_field_paints_its_value() {
    for width in WIDTHS {
        assert_paints(width, "hello", || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(text_field(signal(String::from("hello"))))
        });
    }
}

#[test]
fn checkbox_paints_its_label() {
    for width in WIDTHS {
        assert_paints(width, "enabled", || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(checkbox(signal(true)).label("enabled"))
        });
    }
}

/// Text must survive being squeezed into a `flex: 1 1 0` cell, which is
/// where the vanished dropdown label actually happened.
#[test]
fn text_survives_the_equal_columns_idiom() {
    for width in WIDTHS {
        let painted = painted_text(width, || {
            fn cell(child: impl IntoNode + 'static) -> NodeBuilder {
                container()
                    .layout(LayoutStyle::flex_column().grow(1.0).width(0.0))
                    .child(child)
            }
            container()
                .layout(LayoutStyle::flex_row().fill_width().gap(8.0))
                .child(cell(dropdown(signal(0_usize), vec!["Banana".into()])))
                .child(cell(button("Save")))
        });
        for needle in ["Banana", "Save"] {
            assert!(
                painted.iter().any(|t| t.contains(needle)),
                "at width {width}: {needle:?} vanished inside a flex:1 1 0 cell; \
                 scene painted {painted:?}",
            );
        }
    }
}
