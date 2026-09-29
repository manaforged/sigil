#![allow(clippy::float_cmp)]

use crate::kernel::{Cx, IntoNode, NodeBuilder, container};
use crate::reactive::Runtime;

use super::{SplitterOrientation, splitter};
use crate::geometry::{LogicalSize, Size};
use crate::kernel::TextContent;
use crate::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use crate::widgets::layout_ext::WidgetLayoutExt;

struct Measure;

impl TextMeasure for Measure {
    fn measure(&self, _source: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::default()
    }
}

#[test]
fn fixed_pane_and_handle_keep_their_width_with_filling_content() {
    Runtime::scope("splitter-fixed-pane", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            splitter(SplitterOrientation::Horizontal)
                .pane(container().layout(LayoutStyle::flex_column().fill()))
                .initial(224.0)
                .min(180.0)
                .max(500.0)
                .pane(container().layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0)))
                .grow(1.0)
                .min(320.0)
                .layout(LayoutStyle::flex_row().fill()),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
        layout
            .compute(&mut cx, root, Size::fixed(1280.0, 900.0))
            .expect("layout");
        let first = cx.nth_child(root, 0).expect("sidebar");
        let handle = cx.nth_child(root, 1).expect("handle");
        let second = cx.nth_child(root, 2).expect("content");
        let sidebar = layout.result(first).expect("sidebar bounds").rect();
        let divider = layout.result(handle).expect("divider bounds").rect();
        let content = layout.result(second).expect("content bounds").rect();
        assert_eq!(sidebar.width, 224.0);
        assert_eq!(divider.width, super::HANDLE_PX);
        assert_eq!(divider.origin.x, sidebar.width);
        assert_eq!(content.origin.x, divider.origin.x + divider.width);
        assert_eq!(content.origin.x + content.width, 1280.0);
    });
}

fn two_pane() -> impl IntoNode {
    splitter(SplitterOrientation::Horizontal)
        .child(container(), 120.0)
        .child(container(), 200.0)
}

#[test]
fn helper_returns_into_node() {
    Runtime::scope("splitter-into", || {
        let mut cx = Cx::new_root();
        let id = cx.add(two_pane());
        assert!(cx.get(id).is_some());
    });
}

#[test]
fn splitter_converts_to_node_builder() {
    Runtime::scope("splitter-from", || {
        let node: NodeBuilder = splitter(SplitterOrientation::Horizontal)
            .child(container(), 120.0)
            .child(container(), 200.0)
            .into();
        let mut cx = Cx::new_root();
        let id = cx.add(node);
        assert!(cx.get(id).is_some());
    });
}

#[test]
fn pane_builder_converts_to_node_builder() {
    Runtime::scope("pane-from", || {
        let node: NodeBuilder = splitter(SplitterOrientation::Horizontal)
            .pane(container())
            .initial(120.0)
            .pane(container())
            .grow(1.0)
            .into();
        let mut cx = Cx::new_root();
        let id = cx.add(node);
        assert!(cx.get(id).is_some());
    });
}
