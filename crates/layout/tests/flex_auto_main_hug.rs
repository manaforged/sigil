//! A flex container with an AUTO main size shrinks to fit its content,
//! even inside a parent shorter than that content.
//!
//! Regression: an auto-height flex column inside a shorter parent (a scroll
//! viewport especially) used to FILL the parent's main extent and then §9.7
//! shrink its items to fit — crushing stacked rows on top of each other. It
//! must instead size to its content and overflow the parent (which clips /
//! scrolls). Items that fill via `grow` or an explicit size are unaffected.

#![allow(missing_docs, unused_crate_dependencies, clippy::unwrap_used)]

use sigil_hal::Size;
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_layout::{LayoutEngine, TextMeasure};

struct NoText;
impl TextMeasure for NoText {
    fn measure(&self, _: &sigil_kernel::TextContent, _: Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(0.0, 0.0)
    }
}

/// Height of an auto-height flex column of three 50px rows (150px content),
/// placed inside a parent whose own height is `parent_h`.
fn column_height(parent_h: f32) -> f32 {
    use sigil_kernel::{Cx, container};
    let mut cx = Cx::new_root();
    let rows = |c: sigil_kernel::NodeBuilder| {
        c.child(container().layout(LayoutStyle::block().height(50.0).fill_width()))
            .child(container().layout(LayoutStyle::block().height(50.0).fill_width()))
            .child(container().layout(LayoutStyle::block().height(50.0).fill_width()))
    };
    let col = rows(container().layout(LayoutStyle::flex_column().fill_width()));
    // Parent is a fixed-height block SHORTER than the 150px content.
    let parent = container()
        .layout(LayoutStyle::block().height(parent_h).width(300.0))
        .child(col);
    let root = cx.add(parent);
    let mut layout = LayoutEngine::with_text_measure(Box::new(NoText));
    layout
        .compute(&mut cx, root, Size::fixed(300.0, 800.0))
        .unwrap();
    let c = *cx.get(root).unwrap().children().first().unwrap();
    layout.result(c).unwrap().rect().height
}

#[test]
fn auto_main_column_hugs_content_in_a_shorter_parent() {
    // Parent 60px, content 150px: the column must keep its 150px content,
    // not crush to 60 (which would overlap its rows).
    let h = column_height(60.0);
    assert!(
        (h - 150.0).abs() < 0.5,
        "auto-height column must size to its 150px content, not crush to the \
         60px parent; got {h}",
    );
}
