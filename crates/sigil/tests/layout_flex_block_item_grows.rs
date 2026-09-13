#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::hal::Size;
use sigil::kernel::{Cx, NodeId, TextContent, container};
use sigil::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil::layout::{LayoutEngine, TextMeasure};

struct NoText;
impl TextMeasure for NoText {
    fn measure(&self, _c: &TextContent, _s: Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(0.0, 0.0)
    }
}

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
