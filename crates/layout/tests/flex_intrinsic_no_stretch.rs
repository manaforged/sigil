#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

//! Regressions for the Q11 layout bug:
//!
//! 1. `item_main_basis` for column parents must pass axis-relative
//!    `(cb_main, cb_cross)` to `container_intrinsic_main`, not raw
//!    `(ctx.cb.w, ctx.cb.h)`. The raw form swapped the axes for
//!    column parents — children got laid out at
//!    `(parent_height, parent_width)` and the basis came back as the
//!    parent's WIDTH instead of the child's natural HEIGHT.
//!
//! 2. The hypothetical layout inside `container_intrinsic_main` and
//!    `container_intrinsic_cross` must suppress `align-items:
//!    stretch`. With stretch on, an empty container child inflates
//!    to fill its parent's cross axis; the OUTER read then sees that
//!    as the OUTER's "natural" main extent and over-reports its
//!    basis by the full parent dimension.
//!
//! Combined symptom: a `patterns::field` `flex_column` with a text
//! label + a `flex_row` container child collapsed the label to ~1.6 px
//! and stretched the container to ~58 px instead of leaving them at
//! basis 16/16 with 24 px unused.

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;

mod stub;
use stub::stub_engine;

use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

fn rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("rect")
        .rect()
}

#[test]
fn column_with_text_and_container_with_text_child_does_not_collapse_text() {
    // Reproducer for the `patterns::field("chip (all 5 tones)", chips)`
    // shape: column-direction parent, leaf text + container holding
    // its own text child. Both should keep their basis 16, sit
    // stacked with the column's gap, and leave the rest of the
    // parent's content height unused.
    let mut cx = Cx::new_root();
    let row_child = container()
        .layout(LayoutStyle::flex_row())
        .child(text("nested".to_string()));
    let root = cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(600.0, 80.0)
                    .padding(LengthPercentage::Px(8.0))
                    .gap(LengthPercentage::Px(4.0)),
            )
            .child(text("first".to_string()))
            .child(row_child),
    );
    let first = *cx.get(root).unwrap().children().first().unwrap();
    let inner = *cx.get(root).unwrap().children().get(1).unwrap();

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(600.0, 80.0))
        .expect("compute");

    let first_rect = rect(&cx, first);
    let inner_rect = rect(&cx, inner);
    assert_eq!(
        first_rect.height, 16.0,
        "text leaf must keep its intrinsic 16 height; got {}",
        first_rect.height,
    );
    assert_eq!(
        inner_rect.height, 16.0,
        "row container with text child must keep its intrinsic 16 height; got {}",
        inner_rect.height,
    );
    assert!(
        inner_rect.origin.y >= first_rect.origin.y + first_rect.height + 3.5,
        "inner row must sit below `first` with the column's 4 px gap; got y={} after y={} (h={})",
        inner_rect.origin.y,
        first_rect.origin.y,
        first_rect.height,
    );
}

#[test]
fn column_with_container_then_text_does_not_collapse_text() {
    // Mirror of the above with the order swapped — symmetric coverage
    // so a future regression that fires only on one ordering surfaces.
    let mut cx = Cx::new_root();
    let row_child = container()
        .layout(LayoutStyle::flex_row())
        .child(text("nested".to_string()));
    let root = cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(600.0, 80.0)
                    .padding(LengthPercentage::Px(8.0))
                    .gap(LengthPercentage::Px(4.0)),
            )
            .child(row_child)
            .child(text("second".to_string())),
    );
    let inner = *cx.get(root).unwrap().children().first().unwrap();
    let second = *cx.get(root).unwrap().children().get(1).unwrap();

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(600.0, 80.0))
        .expect("compute");

    let inner_rect = rect(&cx, inner);
    let second_rect = rect(&cx, second);
    assert_eq!(inner_rect.height, 16.0);
    assert_eq!(second_rect.height, 16.0);
}

#[test]
fn column_with_text_and_empty_container_child_does_not_collapse_text() {
    // Tightest reproducer: text leaf + container holding an EMPTY
    // container. Without the suppress-stretch fix, the empty
    // grandchild inflated to the parent's cross axis and the outer
    // container reported a basis equal to the parent's height.
    let mut cx = Cx::new_root();
    let row_child = container()
        .layout(LayoutStyle::flex_row())
        .child(container().layout(LayoutStyle::flex_row()));
    let root = cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(600.0, 80.0)
                    .padding(LengthPercentage::Px(8.0))
                    .gap(LengthPercentage::Px(4.0)),
            )
            .child(text("first".to_string()))
            .child(row_child),
    );
    let first = *cx.get(root).unwrap().children().first().unwrap();
    let inner = *cx.get(root).unwrap().children().get(1).unwrap();

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(600.0, 80.0))
        .expect("compute");

    let first_rect = rect(&cx, first);
    let inner_rect = rect(&cx, inner);
    assert_eq!(
        first_rect.height, 16.0,
        "text leaf must keep its intrinsic 16 height even when sibling is an empty-tree container; got {}",
        first_rect.height,
    );
    assert_eq!(
        inner_rect.height, 0.0,
        "empty-tree container must report height 0, not the parent's full height; got {}",
        inner_rect.height,
    );
}
