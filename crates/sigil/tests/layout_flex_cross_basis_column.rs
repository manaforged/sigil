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
fn fill_width_in_flex_column_resolves_against_parent_width() {
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

    let r = layout_rect(&cx, child);
    assert_eq!(
        r.width, 220.0,
        "fill_width child in a flex_column parent must take parent's WIDTH (220), got {}",
        r.width,
    );
}

#[test]
fn fill_height_in_flex_row_resolves_against_parent_height() {
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

    let r = layout_rect(&cx, child);
    assert_eq!(
        r.height, 220.0,
        "fill_height child in a flex_row parent must take parent's HEIGHT (220), got {}",
        r.height,
    );
}
