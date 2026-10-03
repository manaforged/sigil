#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;

#[test]
fn stacked_text_in_a_short_column_does_not_crush() {
    sigil::reactive::Runtime::scope("flex-min", || {
        sigil::layout::set_text_geometry(std::rc::Rc::new(sigil::app::EngineGeometry::new()));
        let mut cx = Cx::new_root();
        let title = text("Large title here").font_size(20.0).font_weight(700);
        let sub = text("A smaller subtitle line").font_size(14.0);
        let col = container()
            .layout(LayoutStyle::flex_column().height(20.0))
            .child(title)
            .child(sub);
        let root = cx.add(container().layout(LayoutStyle::flex_column()).child(col));
        let mut layout =
            LayoutEngine::with_text_measure(Box::new(sigil::app::DefaultTextMeasure::new()));
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
