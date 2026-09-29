#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

#[path = "layout_stub/mod.rs"]
mod stub;
use stub::{layout_rect, stub_engine};

use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::*;

#[test]
fn column_with_text_and_container_with_text_child_does_not_collapse_text() {
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

    let first_rect = layout_rect(&cx, first);
    let inner_rect = layout_rect(&cx, inner);
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

    let inner_rect = layout_rect(&cx, inner);
    let second_rect = layout_rect(&cx, second);
    assert_eq!(inner_rect.height, 16.0);
    assert_eq!(second_rect.height, 16.0);
}

#[test]
fn column_with_text_and_empty_container_child_does_not_collapse_text() {
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

    let first_rect = layout_rect(&cx, first);
    let inner_rect = layout_rect(&cx, inner);
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
