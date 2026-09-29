#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::geometry::Size;
use sigil::kernel::{Cx, NodeId, container};
use sigil::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
use sigil_kittest::FixedMeasure;

fn row_widths() -> Vec<f32> {
    let mut cx = Cx::new_root();
    let cell = |grow: f32| {
        container()
            .layout(LayoutStyle::flex_row().grow(grow))
            .child(container().layout(LayoutStyle::block().grow(1.0)))
    };
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fill_width())
            .child(cell(1.4))
            .child(cell(0.8))
            .child(container().layout(LayoutStyle::block().width(96.0).height(20.0))),
    );
    let mut layout = LayoutEngine::with_text_measure(Box::new(FixedMeasure::default()));
    layout
        .compute(&mut cx, root, Size::fixed(778.0, 300.0))
        .unwrap();
    let kids: Vec<NodeId> = cx.get(root).unwrap().children().to_vec();
    kids.iter()
        .map(|k| layout.result(*k).unwrap().rect().width)
        .collect()
}

#[test]
fn item_grow_wins_over_child_grow() {
    assert_eq!(
        row_widths(),
        vec![434.0, 248.0, 96.0],
        "items resolve at their own 1.4:0.8 grow ratio; the fixed sibling keeps 96",
    );
}
