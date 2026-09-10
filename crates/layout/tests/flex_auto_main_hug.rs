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

fn column_height(parent_h: f32) -> f32 {
    use sigil_kernel::{Cx, container};
    let mut cx = Cx::new_root();
    let rows = |c: sigil_kernel::NodeBuilder| {
        c.child(container().layout(LayoutStyle::block().height(50.0).fill_width()))
            .child(container().layout(LayoutStyle::block().height(50.0).fill_width()))
            .child(container().layout(LayoutStyle::block().height(50.0).fill_width()))
    };
    let col = rows(container().layout(LayoutStyle::flex_column().fill_width()));
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
    let h = column_height(60.0);
    assert!(
        (h - 150.0).abs() < 0.5,
        "auto-height column must size to its 150px content, not crush to the \
         60px parent; got {h}",
    );
}
