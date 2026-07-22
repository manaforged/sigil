//! Diagnostic — a real `text_field` placed deep in a tree (pushed far down,
//! like the chat composer at the bottom of the window) must render its glyphs
//! at the field, not escape to the window's top-left.
//!
//! Reproduces the reported bug: typing in the composer put the text in the
//! corner of the window. This drives the ACTUAL `text_field` widget (relative
//! region → absolute `position_reactive` track → reactive text + caret overlay)
//! rather than a synthetic layout tree, so it exercises the real structure.
//!
//! Root cause (confirmed by this test): the first full layout positions the
//! field correctly (text at y≈800). Typing then dirties the field's text and
//! rebuilds the caret overlay (`children_reactive` — structural), so the next
//! `compute` takes the incremental path: it re-solves the field's fixed-size
//! `region` as an isolated layout root anchored at (0,0). Without the
//! origin-restore fold in `LayoutEngine::incremental_sync_and_measure`, the
//! whole field — text, track, caret — escapes to the window's top-left.
//!
//! Assertions read the `LayoutResult` ATTACHED to the kernel node, which is the
//! source `renderer/scene.rs` paints from (not the live drive box, which
//! legitimately reads (0,0) for the isolated remeasure root).

#![allow(missing_docs)]
#![allow(
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

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
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::KeyEventInit;
use sigil_layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil_style::TextStyle;

const VIEWPORT: Size = Size::fixed(1000.0, 1000.0);
/// Height of the spacer that pushes the field down — the field's text must land
/// at roughly this y, never near 0.
const SPACER_H: f32 = 800.0;

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

struct StubGeometry;
impl TextGeometry for StubGeometry {
    fn hit(&self, text: &str, _style: &TextStyle, x: f32) -> usize {
        ((x / 10.0).round().max(0.0) as usize).min(text.len())
    }
    fn caret(&self, _text: &str, _style: &TextStyle, byte: usize) -> CaretGeometry {
        CaretGeometry {
            x: byte as f32 * 10.0,
            top: 0.0,
            height: 16.0,
        }
    }
}

#[test]
// Upstream test walks one type-then-measure scenario (60 lines);
// splitting the build/type/assert phases would obscure it.
#[allow(clippy::too_many_lines)]
fn deep_text_field_glyphs_stay_in_field_after_typing() {
    sigil::reactive::Runtime::scope("field-deep-position", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::from("a"));

        // A tall spacer (with filler labels to inflate the node count so typing
        // takes the incremental layout path) pushes the field down to ~SPACER_H.
        let mut spacer =
            container().layout(LayoutStyle::flex_column().fixed_size(1000.0, SPACER_H));
        for i in 0..120 {
            spacer = spacer.child(label(format!("filler row {i}")));
        }
        let tree = container()
            .layout(LayoutStyle::flex_column().fixed_size(1000.0, 1000.0))
            .child(spacer)
            .child(text_field(value));

        let mut cx = Cx::new_root();
        let root = cx.add(tree);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, VIEWPORT)
            .expect("initial layout");

        let field_root = field_text_field_root(&cx, root);
        let field_text = find_text_node(&cx, field_root).expect("field has a text node");
        let y0 = attached_top(&cx, field_text);
        assert!(
            y0 > SPACER_H - 100.0,
            "field text should sit near the field (y≈{SPACER_H}), got y={y0} — \
             static absolute containing-block error",
        );

        // Focus + type: rebuilds the caret overlay (structural) and changes the
        // text content, then re-layout — the incremental path the bug lived on.
        let gen_before = layout.generation();
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(field_root));
        router.route(&mut cx, &layout, type_char("b", PhysicalKey::KeyB));
        layout
            .compute(&mut cx, root, VIEWPORT)
            .expect("layout after typing");

        // Guard against a vacuous pass: typing must have dirtied the tree enough
        // to miss the layout cache and actually run the incremental path. Without
        // this, a wiring regression (focus not landing, key handler unhooked)
        // would leave y1 == y0 and the position asserts would pass for free.
        assert_ne!(
            layout.generation(),
            gen_before,
            "typing did not invalidate layout — incremental path never ran (vacuous test)",
        );

        let field_text2 = find_text_node(&cx, field_root).expect("field still has a text node");
        // Paint path (attached extension, read by renderer/scene.rs).
        let y1 = attached_top(&cx, field_text2);
        assert!(
            y1 > SPACER_H - 100.0,
            "field text escaped after typing (paint): y went {y0} -> {y1} (expected ≈{SPACER_H})",
        );
        // Hit-test path (engine.result → the live drive box, read by
        // hit_test.rs + pointer_dispatch.rs). MUST agree with paint, or clicks
        // miss the field and land on whatever sits at the top-left (GLM MAJOR-1).
        let hit = layout
            .result(field_text2)
            .expect("field text has a drive result")
            .rect();
        assert!(
            hit.origin.y > SPACER_H - 100.0,
            "field text hit-region escaped after typing: drive y={} (expected ≈{SPACER_H}) — \
             paint/hit-test split: clicks would miss the field",
            hit.origin.y,
        );
        // x must survive too (the dx half of the translation, never 0 here:
        // the field is inset from the window's left edge).
        assert!(
            hit.origin.x > 1.0,
            "field text x collapsed to the left edge: drive x={}",
            hit.origin.x,
        );
    });
}

/// The `text_field` root is the tree's second child (after the spacer).
fn field_text_field_root(cx: &Cx, root: NodeId) -> NodeId {
    cx.nth_child(root, 1).expect("field is the second child")
}

fn find_text_node(cx: &Cx, root: NodeId) -> Option<NodeId> {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if found.is_none() && matches!(node.kind, NodeType::Text { .. }) {
            found = Some(id);
        }
    });
    found
}

/// The y-origin the painter actually reads — the `LayoutResult` attached to the
/// kernel node (translated by the incremental fix), not the live drive box.
/// `scene.rs` emits geometry from this attached extension (`get_extension`).
fn attached_top(cx: &Cx, id: NodeId) -> f32 {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("text node has an attached layout result")
        .rect()
        .origin
        .y
}

fn type_char(ch: &str, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(ch.to_string().into()),
    }))
}
