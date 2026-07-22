//! A flex container with an AUTO cross size shrinks to fit its content,
//! even under `align-items: center`.
//!
//! Regression: a single-line flex row with `align-items: center` and no
//! explicit height used to inflate to its containing block's cross extent
//! (the auto fallback in `container_inner_size`), then centre its child in
//! that oversized band — so a ~19px row ballooned to ~114px in a tall
//! parent, and a `text_field` grew to ~173px in a flexible column. The row
//! must keep its content height regardless of `align-items`.

#![allow(missing_docs, unused_crate_dependencies, clippy::unwrap_used)]

use sigil_hal::Size;
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_layout::{LayoutEngine, TextMeasure};

struct NoText;
impl TextMeasure for NoText {
    fn measure(&self, _: &sigil_kernel::TextContent, _: Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(0.0, 0.0)
    }
}

fn row_height(align: AlignItems) -> f32 {
    use sigil_kernel::{Cx, container};
    let mut cx = Cx::new_root();
    // A 20px fixed-height child inside a flex row with the given align, all
    // inside a column laid out in a 400px-tall viewport.
    let child = container().layout(LayoutStyle::block().height(20.0).fill_width());
    let row = container()
        .layout(LayoutStyle::flex_row().fill_width().align_items(align))
        .child(child);
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_column().width(300.0))
            .child(row),
    );
    let mut layout = LayoutEngine::with_text_measure(Box::new(NoText));
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
