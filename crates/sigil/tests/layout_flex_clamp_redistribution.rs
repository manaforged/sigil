#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::hal::Size;
use sigil::kernel::{Cx, NodeId, container};
use sigil::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil::layout::{Dimension, LayoutEngine};
use sigil_kittest::FixedMeasure;

fn widths(root_w: f32, a: LayoutStyle, b: LayoutStyle) -> Vec<f32> {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fill_width())
            .child(container().layout(a))
            .child(container().layout(b)),
    );
    let mut layout = LayoutEngine::with_text_measure(Box::new(FixedMeasure::default()));
    layout
        .compute(&mut cx, root, Size::fixed(root_w, 100.0))
        .unwrap();
    let kids: Vec<NodeId> = cx.get(root).unwrap().children().to_vec();
    kids.iter()
        .map(|k| layout.result(*k).unwrap().rect().width)
        .collect()
}

#[test]
fn grow_max_clamp_redistributes_to_sibling() {
    let a = LayoutStyle::flex_column()
        .grow(1.0)
        .width(0.0)
        .max_size(50.0, Dimension::Auto);
    let b = LayoutStyle::flex_column().grow(1.0).width(0.0);
    assert_eq!(
        widths(400.0, a, b),
        vec![50.0, 350.0],
        "space refused by a max-clamped item must go to its sibling, \
         not be left unallocated",
    );
}

#[test]
fn shrink_min_clamp_redistributes_to_sibling() {
    let a = LayoutStyle::flex_column()
        .width(200.0)
        .shrink(1.0)
        .min_size(150.0, Dimension::Auto);
    let b = LayoutStyle::flex_column().width(200.0).shrink(1.0);
    let got = widths(100.0, a, b);
    assert_eq!(
        got,
        vec![150.0, 0.0],
        "the min-clamped item holds its 150 floor, and the deficit it \
         refused falls entirely on the sibling",
    );
}
