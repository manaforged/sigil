#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

//! Regression: `fill_width()` on a flex-column child must resolve
//! against the parent's WIDTH (cross axis), not its HEIGHT.
//!
//! Before the fix in `item_cross_basis`, the column-direction branch
//! resolved a child's width against `cb_main` (parent height), which
//! silently degraded `fill_width()` to the parent's height value and
//! collapsed every `JustifyContent::SpaceBetween` row whose own width
//! depended on `fill_width()` — visible in the playground as `kv_row`'s
//! label and value overprinting at x=0.

use ahash as _;
use allocation_counter as _;
use criterion as _;
use hal as _;
use rayon as _;
use reactive as _;
use smallvec as _;
use style as _;

mod stub;
use stub::stub_engine;

use kernel::prelude::*;
use layout::prelude::*;

fn child_rect(cx: &Cx, id: NodeId) -> hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("child result")
        .rect()
}

#[test]
fn fill_width_in_flex_column_resolves_against_parent_width() {
    // Parent: flex_column 220 wide × 40 tall (NOT square, so a swapped
    // axis is immediately visible in the assertion).
    // Child: fill_width — expected to resolve to 220, NOT 40.
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container()));
    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_column().fixed_size(220.0, 40.0));
    let child = cx
        .get(root)
        .expect("root exists")
        .children()
        .first()
        .copied()
        .expect("child exists");
    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::flex_row().fill_width());

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(220.0, 40.0))
        .expect("compute succeeds");

    let r = child_rect(&cx, child);
    assert_eq!(
        r.width, 220.0,
        "fill_width child in a flex_column parent must take parent's WIDTH (220), got {}",
        r.width,
    );
}

#[test]
fn fill_height_in_flex_row_resolves_against_parent_height() {
    // Mirror: a row parent with fill_height child. This case worked
    // before the fix (the row-direction branch already used cb_cross
    // correctly) — included so any future regression on the row side
    // surfaces alongside the column case.
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container()));
    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_row().fixed_size(40.0, 220.0));
    let child = cx
        .get(root)
        .expect("root exists")
        .children()
        .first()
        .copied()
        .expect("child exists");
    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::flex_column().fill_height());

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(40.0, 220.0))
        .expect("compute succeeds");

    let r = child_rect(&cx, child);
    assert_eq!(
        r.height, 220.0,
        "fill_height child in a flex_row parent must take parent's HEIGHT (220), got {}",
        r.height,
    );
}
