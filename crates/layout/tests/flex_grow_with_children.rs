#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
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

fn two_cells(with_child: bool) -> Vec<f32> {
    let mut cx = Cx::new_root();
    let cell = || {
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
