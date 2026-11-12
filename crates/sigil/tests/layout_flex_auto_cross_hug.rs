#![allow(clippy::unwrap_used)]

use sigil::geometry::Constraint;
use sigil::layout::{AlignItems, LayoutEngine, LayoutStyle};
use sigil::widgets::Layout;
use sigil_kittest::FakeMeasure;

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
    let mut layout = LayoutEngine::new(Box::new(FakeMeasure::default()));
    layout
        .compute(&mut cx, root, Constraint::fixed(300.0, 400.0))
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
