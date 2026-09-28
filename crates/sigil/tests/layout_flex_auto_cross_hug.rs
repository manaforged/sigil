#![allow(unused_crate_dependencies, clippy::unwrap_used)]

use sigil::hal::Size;
use sigil::layout::LayoutEngine;
use sigil::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_kittest::FixedMeasure;

fn row_height(align: AlignItems) -> f32 {
    use sigil::kernel::{Cx, container};
    let mut cx = Cx::new_root();
    let child = container().layout(LayoutStyle::block().height(20.0).fill_width());
    let row = container()
        .layout(LayoutStyle::flex_row().fill_width().align_items(align))
        .child(child);
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_column().width(300.0))
            .child(row),
    );
    let mut layout = LayoutEngine::with_text_measure(Box::new(FixedMeasure::default()));
    layout
        .compute(&mut cx, root, Size::fixed(300.0, 400.0))
        .unwrap();
    let r = *cx.get(root).unwrap().children().first().unwrap();
    layout.result(r).unwrap().rect().height
}

#[test]
fn auto_cross_row_hugs_content_under_center_align() {
    let center = row_height(AlignItems::Center);
    let start = row_height(AlignItems::Start);
    assert!(
        (center - 20.0).abs() < 0.5,
        "align:center row must hug its 20px child, not inflate to the parent; got {center}",
    );
    assert!(
        (start - 20.0).abs() < 0.5,
        "align:start row must also hug; got {start}",
    );
}
