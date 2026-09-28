#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

#[path = "layout_stub/mod.rs"]
mod stub;

use sigil::hal::Size;
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;
use sigil::reactive::{Runtime, signal};
use sigil::style::{Dimension, DisplayKind, NodeStyleExt, Px, StyleAtom};
use sigil_kittest::CharMeasure;
use stub::{layout_rect, stub_engine};

const AVAILABLE: Size = Size::fixed(120.0, 40.0);
static NARROW: &[StyleAtom] = &[StyleAtom::Width(Dimension::Px(Px(32)))];
static WIDE: &[StyleAtom] = &[StyleAtom::Width(Dimension::Px(Px(96)))];
static HIDDEN: &[StyleAtom] = &[StyleAtom::Display(DisplayKind::None)];

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
        .and_then(|node| node.get_extension::<LayoutResult>())
        .is_some()
}

fn layout_result(cx: &Cx, id: NodeId) -> LayoutResult {
    *cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
}

const TEXT_MEASURE: CharMeasure = CharMeasure {
    char_width: 10.0,
    line_height: 10.0,
};

#[path = "layout_incremental/cache.rs"]
mod cache;
#[path = "layout_incremental/incremental.rs"]
mod incremental;
#[path = "layout_incremental/reactive.rs"]
mod reactive;
