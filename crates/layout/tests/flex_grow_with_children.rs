//! Regression: a flex item that has children must still grow.
//!
//! `write_container` re-derives a container's box after laying out its
//! children. It used to shrink-wrap the cross axis to content, which
//! silently discarded the extent the parent's flexible-length pass had
//! already resolved — so `grow(1)` collapsed back to the content width
//! the moment the item gained a child. Childless items return early from
//! `layout_item_subtree` and never reach that overwrite, which is why the
//! existing flex tests (all childless leaves) stayed green.
//!
//! This is the `flex: 1 1 0` equal-columns idiom, so the failure hit
//! essentially every real composed layout.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used)]

use sigil_hal::Size;
use sigil_kernel::{Cx, NodeId, TextContent, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_layout::{LayoutEngine, TextMeasure};

struct NoText;
impl TextMeasure for NoText {
    fn measure(&self, _c: &TextContent, _s: Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(0.0, 0.0)
    }
}

/// Two `grow(1)` cells in a 400px row with a 12px gap: 194 each.
fn two_cells(with_child: bool) -> Vec<f32> {
    let mut cx = Cx::new_root();
    let cell = || {
        // `flex: 1 1 0` as authored in real code: grow + a zero basis
        // expressed via `width(0)`. The flexed size must win over the
        // specified zero.
        let node = container().layout(LayoutStyle::flex_column().grow(1.0).width(0.0));
        if with_child {
            node.child(container().layout(LayoutStyle::flex_column().width(148.0).height(20.0)))
        } else {
            node
        }
    };
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fill_width().gap(12.0))
            .child(cell())
            .child(cell()),
    );
    let mut layout = LayoutEngine::with_text_measure(Box::new(NoText));
    layout
        .compute(&mut cx, root, Size::fixed(400.0, 300.0))
        .unwrap();
    let kids: Vec<NodeId> = cx.get(root).unwrap().children().to_vec();
    kids.iter()
        .map(|k| layout.result(*k).unwrap().rect().width)
        .collect()
}

#[test]
fn grow_cell_fills_free_space_whether_or_not_it_has_children() {
    assert_eq!(two_cells(false), vec![194.0, 194.0], "childless cells grow");
    assert_eq!(
        two_cells(true),
        vec![194.0, 194.0],
        "a cell with a 148px child must still grow to 194, not \
         shrink-wrap back to its content width",
    );
}
