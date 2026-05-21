#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

//! Phase D — `LayoutStyle::fill()` acceptance tests.
//!
//! Verifies the new `Dimension::Fill` variant resolves against the
//! parent's resolved content-box during the layout pass. Covers the
//! brief's four acceptance cases:
//!
//! 1. `fill_resolves_to_parent_content_box` — parent 800×600 + Fill
//!    child → 800×600.
//! 2. `fill_honours_parent_padding` — parent 800×600 with padding(20)
//!    + Fill child → 760×560.
//! 3. `fill_axis_specific` — parent 800×600 + `fill_width().height(100)`
//!    → 800×100.
//! 4. `fill_independent_of_width_height_builders` — `LayoutStyle::fill()`
//!    is a shorthand for `fill_width().fill_height()`.

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
fn fill_resolves_to_parent_content_box() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container()));
    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::block().fixed_size(800.0, 600.0));
    let child = cx
        .get(root)
        .expect("root exists")
        .children()
        .first()
        .copied()
        .expect("child exists");
    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::block().fill());

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(800.0, 600.0))
        .expect("compute succeeds");

    let r = child_rect(&cx, child);
    assert_eq!(r.width, 800.0, "Fill child width = parent content width");
    assert_eq!(r.height, 600.0, "Fill child height = parent content height");
}

#[test]
fn fill_honours_parent_padding() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container()));
    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::block()
            .fixed_size(800.0, 600.0)
            .padding(LengthPercentage::Px(20.0)),
    );
    let child = cx
        .get(root)
        .expect("root exists")
        .children()
        .first()
        .copied()
        .expect("child exists");
    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::block().fill());

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(800.0, 600.0))
        .expect("compute succeeds");

    let r = child_rect(&cx, child);
    assert_eq!(
        r.width, 760.0,
        "Fill child width = parent (800) - 2 * padding (20)",
    );
    assert_eq!(
        r.height, 560.0,
        "Fill child height = parent (600) - 2 * padding (20)",
    );
}

#[test]
fn fill_axis_specific() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container()));
    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::block().fixed_size(800.0, 600.0));
    let child = cx
        .get(root)
        .expect("root exists")
        .children()
        .first()
        .copied()
        .expect("child exists");
    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::block().fill_width().height(100.0));

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(800.0, 600.0))
        .expect("compute succeeds");

    let r = child_rect(&cx, child);
    assert_eq!(r.width, 800.0, "fill_width inherits parent content width");
    assert_eq!(r.height, 100.0, "height(100) takes explicit value");
}

#[test]
fn fill_height_axis_specific() {
    // Symmetric counterpart to `fill_axis_specific` — verify the
    // block-axis builder works independently of the inline axis.
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container()));
    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::block().fixed_size(800.0, 600.0));
    let child = cx
        .get(root)
        .expect("root exists")
        .children()
        .first()
        .copied()
        .expect("child exists");
    cx.get_mut(child)
        .expect("child exists")
        .attach(LayoutStyle::block().width(120.0).fill_height());

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(800.0, 600.0))
        .expect("compute succeeds");

    let r = child_rect(&cx, child);
    assert_eq!(r.width, 120.0, "width(120) takes explicit value");
    assert_eq!(
        r.height, 600.0,
        "fill_height inherits parent content height",
    );
}

#[test]
fn fill_shorthand_matches_fill_width_fill_height() {
    // `fill()` must produce identical resolved sizes to chaining
    // `fill_width().fill_height()`. This is a builder-shape contract:
    // the shorthand is sugar, not a separate code path.
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .child(container()) // sibling-a uses fill()
            .child(container()), // sibling-b uses fill_width().fill_height()
    );
    cx.get_mut(root).expect("root exists").attach(
        LayoutStyle::flex_row() // row container so children sit side-by-side
            .fixed_size(800.0, 600.0),
    );
    let kids: Vec<NodeId> = cx.get(root).expect("root exists").children().to_vec();
    cx.get_mut(kids[0])
        .expect("a exists")
        .attach(LayoutStyle::block().fill());
    cx.get_mut(kids[1])
        .expect("b exists")
        .attach(LayoutStyle::block().fill_width().fill_height());

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(800.0, 600.0))
        .expect("compute succeeds");

    let ra = child_rect(&cx, kids[0]);
    let rb = child_rect(&cx, kids[1]);
    assert_eq!(
        (ra.width, ra.height),
        (rb.width, rb.height),
        "fill() and fill_width().fill_height() resolve to the same size",
    );
}
