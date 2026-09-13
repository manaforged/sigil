#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use sigil::hal::KeyEventInit;
use sigil::layout::{CaretGeometry, TextGeometry, set_text_geometry};
use sigil::prelude::*;
use sigil::style::TextStyle;

const VIEWPORT: Size = Size::fixed(1000.0, 1000.0);
const SPACER_H: f32 = 800.0;

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
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
#[allow(clippy::too_many_lines)]
fn deep_text_field_glyphs_stay_in_field_after_typing() {
    sigil::reactive::Runtime::scope("field-deep-position", || {
        set_text_geometry(std::rc::Rc::new(StubGeometry));
        let value = signal(String::from("a"));

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

        let gen_before = layout.generation();
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(field_root));
        router.route(&mut cx, &layout, type_char("b", PhysicalKey::KeyB));
        layout
            .compute(&mut cx, root, VIEWPORT)
            .expect("layout after typing");

        assert_ne!(
            layout.generation(),
            gen_before,
            "typing did not invalidate layout — incremental path never ran (vacuous test)",
        );

        let field_text2 = find_text_node(&cx, field_root).expect("field still has a text node");
        let y1 = attached_top(&cx, field_text2);
        assert!(
            y1 > SPACER_H - 100.0,
            "field text escaped after typing (paint): y went {y0} -> {y1} (expected ≈{SPACER_H})",
        );
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
        assert!(
            hit.origin.x > 1.0,
            "field text x collapsed to the left edge: drive x={}",
            hit.origin.x,
        );
    });
}

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
