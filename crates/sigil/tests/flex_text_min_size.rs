//! CSS Flexbox §4.5 automatic minimum size for content leaves.
//!
//! A text leaf in a flex column that is shorter than its content must not
//! shrink below the text's own height. Before this fix the shrink pass
//! crushed stacked text toward zero, so a large title and its subtitle
//! painted on top of each other (found via the widget-playground render:
//! layout allocated ~5px for a 24px title). This pins the floor: two
//! stacked text nodes in an over-constrained column keep their measured
//! heights and stay vertically separated.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;
use wgpu as _;

use sigil::prelude::*;

#[test]
fn stacked_text_in_a_short_column_does_not_crush() {
    sigil::reactive::Runtime::scope("flex-min", || {
        sigil_layout::set_text_geometry(std::rc::Rc::new(sigil_app::EngineGeometry::new()));
        let mut cx = Cx::new_root();
        // A column pinned to 20px — far shorter than its two text children
        // stacked (a large title + a smaller subtitle ~40px total). With
        // default shrink and no §4.5 floor, both collapsed toward 0.
        let title = text("Large title here").class("text-xl font-bold");
        let sub = text("A smaller subtitle line").class("text-sm");
        let col = container()
            .layout(LayoutStyle::flex_column().height(20.0))
            .child(title)
            .child(sub);
        let root = cx.add(container().layout(LayoutStyle::flex_column()).child(col));
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil_app::DefaultTextMeasure::new()));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 400.0))
            .unwrap();

        let col_id = *cx.get(root).unwrap().children().first().unwrap();
        let kids: Vec<NodeId> = cx.get(col_id).unwrap().children().to_vec();
        let title_rect = layout.result(*kids.first().unwrap()).unwrap().rect();
        let sub_rect = layout.result(*kids.get(1).unwrap()).unwrap().rect();

        assert!(
            title_rect.height >= 18.0,
            "title text must keep ~its measured height, not be crushed; got {}",
            title_rect.height,
        );
        assert!(
            sub_rect.origin.y >= title_rect.origin.y + title_rect.height,
            "subtitle must sit below the title, not overlap it; title y={} h={} sub y={}",
            title_rect.origin.y,
            title_rect.height,
            sub_rect.origin.y,
        );
    });
}
