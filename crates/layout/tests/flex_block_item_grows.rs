//! A `Display::Block` flex item must honour the width flex resolved for it.
//!
//! `layout_item_subtree` dispatches a block-display child to `BlockSolver`,
//! which re-derived the width from the node's own style. A block item with
//! `grow(1).width(0)` therefore collapsed to zero, while the same node with
//! `display: flex` grew correctly.
//!
//! This is what broke the widget playground once widgets started filling
//! their parents: the scroll viewport's content wrapper is a block item that
//! grows into the space left by the scrollbar gutter, so the entire page
//! collapsed to nothing.

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

/// A `grow(1).width(0)` item beside a fixed 8px sibling, in a 400px row.
/// The grower must take the remaining 392.
fn grower_width(block: bool) -> f32 {
    let mut cx = Cx::new_root();
    let base = if block {
        LayoutStyle::block()
    } else {
        LayoutStyle::flex_column()
    };
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fill_width().height(50.0))
            .child(
                container()
                    .layout(base.grow(1.0).width(0.0).height(50.0))
                    // A child is required: childless items return early from
                    // `layout_item_subtree` and never reach the re-derivation.
                    .child(container().layout(LayoutStyle::block().height(10.0))),
            )
            .child(container().layout(LayoutStyle::block().width(8.0).height(50.0))),
    );
    let mut layout = LayoutEngine::with_text_measure(Box::new(NoText));
    layout
        .compute(&mut cx, root, Size::fixed(400.0, 50.0))
        .unwrap();
    let first: NodeId = *cx.get(root).unwrap().children().first().unwrap();
    layout.result(first).unwrap().rect().width
}

#[test]
fn block_and_flex_items_grow_alike() {
    assert!(
        (grower_width(false) - 392.0).abs() < 0.01,
        "flex-display grower takes the space left by the 8px sibling; got {}",
        grower_width(false),
    );
    assert!(
        (grower_width(true) - 392.0).abs() < 0.01,
        "a BLOCK-display grower must too — it used to collapse to 0 because \
         BlockSolver re-derived the width from its own style; got {}",
        grower_width(true),
    );
}
