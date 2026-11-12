#![allow(
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use crate::widget_support;

use sigil::events::Event;
use sigil::events::EventRouter;
use sigil::events::PhysicalKey;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::{LayoutEngine, LayoutResult, text_geometry};
use sigil::prelude::*;
use sigil_kittest::stub_text_measure;
use widget_support::find_text_node;

const VIEWPORT: Size = Size::fixed(1000.0, 1000.0);
const SPACER_H: f32 = 800.0;

#[test]
#[allow(clippy::too_many_lines)]
fn deep_text_field_glyphs_stay_in_field_after_typing() {
    sigil::reactive::Owner::scope("field-deep-position", || {
        text_geometry(std::rc::Rc::new(sigil::layout::FakeMeasure::default()));
        let value = signal(String::from("a"));

        let mut spacer =
            container().layout(LayoutStyle::flex_column().fixed_size(1000.0, SPACER_H));
        for i in 0..120 {
            spacer = spacer.child(text(format!("filler row {i}")).nowrap());
        }
        let tree = container()
            .layout(LayoutStyle::flex_column().fixed_size(1000.0, 1000.0))
            .child(spacer)
            .child(text_field(value));

        let mut cx = Cx::new_root();
        let root = cx.add(tree);
        let mut layout = LayoutEngine::new(stub_text_measure());
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
        router.route(
            &mut cx,
            &layout,
            Event::typed("b").physical(PhysicalKey::KeyB),
        );
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

fn attached_top(cx: &Cx, id: NodeId) -> f32 {
    cx.get(id)
        .and_then(|node| node.extension::<LayoutResult>())
        .expect("text node has an attached layout result")
        .rect()
        .origin
        .y
}
