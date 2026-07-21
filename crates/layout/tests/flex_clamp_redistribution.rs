//! CSS Flexbox §9.7 "Resolve the flexible lengths" is a LOOP.
//!
//! Distributing free space once and then clamping to min/max strands the
//! space a violating item refused. The spec instead freezes the violating
//! items and redistributes the remainder to the survivors, repeating
//! until nothing violates.
//!
//! The single-pass version passed every pre-existing flex test because
//! those tests either use one child or never clamp — the deviation only
//! fires when a *flexible* item hits a min/max bound, which is exactly
//! what a layout full of `max-w-*` cards does.
//!
//! Cases below were surfaced independently by the the reviewer and Kimi
//! production-readiness reviews (2026-07-20).

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]

use sigil_hal::Size;
use sigil_kernel::{Cx, NodeId, TextContent, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_layout::{Dimension, LayoutEngine, TextMeasure};

struct NoText;
impl TextMeasure for NoText {
    fn measure(&self, _c: &TextContent, _s: Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(0.0, 0.0)
    }
}

fn widths(root_w: f32, a: LayoutStyle, b: LayoutStyle) -> Vec<f32> {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fill_width())
            .child(container().layout(a))
            .child(container().layout(b)),
    );
    let mut layout = LayoutEngine::with_text_measure(Box::new(NoText));
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
    // 400px, two `grow:1` items with basis 0, A capped at 50.
    // A freezes at 50; B must absorb the whole remaining 350.
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
    // 100px container, two 200px items, A floored at 150.
    // A freezes at 150; B must absorb the entire remaining deficit.
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
