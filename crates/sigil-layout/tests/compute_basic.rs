#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;
use taffy as _;
mod stub;
use stub::stub_engine;

use sigil_core::prelude::*;
use sigil_layout::prelude::*;

#[test]
fn flex_row_text_children_use_placeholder_intrinsic_widths() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .child(text("A"))
            .child(text("BB"))
            .child(text("CCC")),
    );
    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_row()
            .fixed_size(300.0, 40.0)
            .align_items(AlignItems::FlexStart),
    );

    let children = cx
        .get(root)
        .expect("root exists")
        .children
        .iter()
        .copied()
        .collect::<Vec<_>>();

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(300.0, 40.0))
        .expect("compute succeeds");

    let widths = children
        .iter()
        .copied()
        .map(|id| {
            cx.get(id)
                .and_then(|node| node.get_extension::<LayoutResult>())
                .expect("child result")
                .rect()
                .width
        })
        .collect::<Vec<_>>();

    assert_eq!(widths, vec![8.0, 16.0, 24.0]);

    let origins = children
        .iter()
        .copied()
        .map(|id| {
            cx.get(id)
                .and_then(|node| node.get_extension::<LayoutResult>())
                .expect("child result")
                .rect()
                .origin
                .x
        })
        .collect::<Vec<_>>();

    assert_eq!(origins, vec![0.0, 8.0, 24.0]);
}

#[test]
fn compute_clears_results_for_nodes_detached_from_root_subtree() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(text("stale")));
    let child = cx
        .get(root)
        .and_then(|node| node.children.first())
        .copied()
        .expect("child exists");

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(100.0, 40.0))
        .expect("initial compute succeeds");

    assert!(
        cx.get(child)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .is_some(),
        "child receives initial layout result",
    );

    cx.get_mut(root).expect("root exists").children.clear();
    cx.get_mut(child).expect("child exists").parent = None;
    cx.mark_mutated();

    engine
        .compute(&mut cx, root, Size::fixed(100.0, 40.0))
        .expect("second compute succeeds");

    assert_eq!(
        engine.result(child),
        None,
        "detached child is no longer in latest computed result set",
    );
    assert!(
        cx.get(child)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .is_none(),
        "detached child projection is cleared from node extensions",
    );
}
