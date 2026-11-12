#![allow(clippy::unwrap_used)]

use crate::layout_stub as stub;

use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::*;
use sigil::prelude::Widget;
use sigil::reactive::{Owner, signal};
use sigil::widgets::Layout;
use sigil_kittest::FakeMeasure;
use stub::{layout_rect, stub_engine};

const AVAILABLE: Size = Size::fixed(120.0, 40.0);

fn build_tree() -> (Cx, NodeId, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(120.0, 40.0))
            .child(container().layout(LayoutStyle::block().fixed_size(12.0, 10.0))),
    );
    let child = cx
        .get(root)
        .and_then(|node| node.children().first().copied())
        .expect("child exists");
    (cx, root, child)
}

fn build_many_leaf_row(count: usize) -> (Cx, NodeId, Vec<NodeId>) {
    let mut cx = Cx::new_root();
    let root = cx.add(container().layout(LayoutStyle::flex_row().fixed_size(200.0, 40.0)));
    let children = (0..count)
        .map(|_| {
            cx.add_child(
                root,
                container().layout(LayoutStyle::block().fixed_size(12.0, 10.0)),
            )
        })
        .collect();
    (cx, root, children)
}

fn ordered_fixed(width: f32, height: f32, order: i32) -> LayoutStyle {
    LayoutStyle::block()
        .fixed_size(width, height)
        .with_layout(|style| style.order = order)
}

fn layout_result_exists(cx: &Cx, id: NodeId) -> bool {
    cx.get(id)
        .and_then(|node| node.extension::<LayoutResult>())
        .is_some()
}

fn layout_result(cx: &Cx, id: NodeId) -> LayoutResult {
    *cx.get(id)
        .and_then(|node| node.extension::<LayoutResult>())
        .expect("layout result exists")
}

const TEXT_MEASURE: FakeMeasure = FakeMeasure::chars(10.0, 10.0);

#[path = "layout_incremental/cache.rs"]
mod cache;
#[path = "layout_incremental/incremental.rs"]
mod incremental;
#[path = "layout_incremental/reactive.rs"]
mod reactive;
