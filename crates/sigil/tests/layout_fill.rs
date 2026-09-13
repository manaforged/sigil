#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]

#[path = "layout_stub/mod.rs"]
mod stub;
use stub::stub_engine;

use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;

fn child_rect(cx: &Cx, id: NodeId) -> sigil::hal::Rect {
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
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container()).child(container()));
    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_row().fixed_size(800.0, 600.0));
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
