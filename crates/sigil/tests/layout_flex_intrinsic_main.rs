#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::doc_markdown, clippy::float_cmp, clippy::unwrap_used)]

#[path = "layout_stub/mod.rs"]
mod stub;
use stub::stub_engine;

use sigil::geometry::{Rect, Size};
use sigil::kernel::*;
use sigil::layout::*;

const AVAILABLE: Size = Size::fixed(800.0, 1000.0);

const TEXT_H: f32 = 16.0;

fn rect_of(cx: &Cx, id: NodeId) -> Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result must exist")
        .rect()
}

fn child_rect(cx: &Cx, parent: NodeId, n: usize) -> Rect {
    let id = *cx
        .get(parent)
        .expect("parent alive")
        .children()
        .get(n)
        .expect("child index in range");
    rect_of(cx, id)
}

fn build_and_compute(
    outer: LayoutStyle,
    inner: &LayoutStyle,
    leaf_labels: &[&[&'static str]],
) -> (Cx, NodeId) {
    let mut cx = Cx::new_root();
    let mut parent = container().layout(outer);
    for labels in leaf_labels {
        let mut row = container().layout(inner.clone());
        for label in *labels {
            row = row.child(text(*label));
        }
        parent = parent.child(row);
    }
    let root = cx.add(parent);
    stub_engine()
        .compute(&mut cx, root, AVAILABLE)
        .expect("compute succeeds");
    (cx, root)
}

#[test]
fn flex_column_with_auto_height_container_children_stacks_vertically() {
    let (cx, root) = build_and_compute(
        LayoutStyle::flex_column(),
        &LayoutStyle::flex_row(),
        &[&["A", "B"], &["C", "D"], &["E", "F"]],
    );

    let c0 = child_rect(&cx, root, 0);
    let c1 = child_rect(&cx, root, 1);
    let c2 = child_rect(&cx, root, 2);

    for (i, r) in [c0, c1, c2].iter().enumerate() {
        assert!(
            r.height >= TEXT_H,
            "child {i} height must be ≥ {TEXT_H} (got {})",
            r.height
        );
    }

    assert!(
        c1.origin.y >= c0.origin.y + c0.height,
        "child 1 must start at or after child 0's bottom (c0.y={}, c0.h={}, c1.y={})",
        c0.origin.y,
        c0.height,
        c1.origin.y,
    );
    assert!(
        c2.origin.y >= c1.origin.y + c1.height,
        "child 2 must start at or after child 1's bottom (c1.y={}, c1.h={}, c2.y={})",
        c1.origin.y,
        c1.height,
        c2.origin.y,
    );
}

#[test]
fn flex_row_with_auto_width_container_children_distributes_horizontally() {
    let (cx, root) = build_and_compute(
        LayoutStyle::flex_row(),
        &LayoutStyle::flex_column(),
        &[&["AA", "BB"], &["CC", "DD"], &["EE", "FF"]],
    );

    let c0 = child_rect(&cx, root, 0);
    let c1 = child_rect(&cx, root, 1);
    let c2 = child_rect(&cx, root, 2);

    for (i, r) in [c0, c1, c2].iter().enumerate() {
        assert!(
            r.width >= 16.0,
            "child {i} width must be ≥ 16 (got {})",
            r.width
        );
    }

    assert!(
        c1.origin.x >= c0.origin.x + c0.width,
        "child 1 must start at or after child 0's right edge (c0.x={}, c0.w={}, c1.x={})",
        c0.origin.x,
        c0.width,
        c1.origin.x,
    );
    assert!(
        c2.origin.x >= c1.origin.x + c1.width,
        "child 2 must start at or after child 1's right edge (c1.x={}, c1.w={}, c2.x={})",
        c1.origin.x,
        c1.width,
        c2.origin.x,
    );
}

#[test]
fn nested_flex_column_in_flex_column_propagates_intrinsic() {
    let (cx, root) = build_and_compute(
        LayoutStyle::flex_column(),
        &LayoutStyle::flex_column(),
        &[&["L1", "L2", "L3"]],
    );

    let inner = child_rect(&cx, root, 0);
    let min = TEXT_H * 3.0;
    assert!(
        inner.height >= min,
        "inner column height must be ≥ {min} (sum of inner text heights), got {}",
        inner.height,
    );
}
