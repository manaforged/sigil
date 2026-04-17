#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

mod stub;

use ahash as _;
use criterion as _;
use sigil_core::prelude::*;
use sigil_hal as _;
use sigil_layout::prelude::*;
use sigil_style::NodeStyleExt;
use smallvec as _;
use taffy as _;

use stub::stub_engine;

#[test]
fn p_4_applies_sixteen_px_padding() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(100.0, 50.0))
            .class("p-4")
            .child(container().class("w-full h-full")),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(100.0, 50.0))
        .expect("compute succeeds");

    let rect = layout_rect(&cx, child);
    assert_eq!(rect.origin.x, 16.0);
    assert_eq!(rect.origin.y, 16.0);
    assert_eq!(rect.width, 68.0);
    assert_eq!(rect.height, 18.0);
}

#[test]
fn w_full_is_percent_100() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 40.0))
            .child(
                container()
                    .layout(LayoutStyle::default().height_px(10.0))
                    .class("w-full"),
            ),
    );
    let child = first_child(&cx, root);

    stub_engine()
        .compute(&mut cx, root, Size::fixed(200.0, 40.0))
        .expect("compute succeeds");

    assert_eq!(layout_rect(&cx, child).width, 200.0);
}

#[test]
fn flex_row_items_center() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::default().fixed_size(100.0, 40.0))
            .class("flex flex-row items-center")
            .child(container().layout(LayoutStyle::default().fixed_size(10.0, 10.0)))
            .child(container().layout(LayoutStyle::default().fixed_size(20.0, 10.0))),
    );
    let children = cx
        .get(root)
        .unwrap()
        .children
        .iter()
        .copied()
        .collect::<Vec<_>>();

    stub_engine()
        .compute(&mut cx, root, Size::fixed(100.0, 40.0))
        .expect("compute succeeds");

    let first = layout_rect(&cx, *children.first().unwrap());
    let second = layout_rect(&cx, *children.get(1).unwrap());

    assert_eq!(first.origin.x, 0.0);
    assert_eq!(first.origin.y, 15.0);
    assert_eq!(second.origin.x, 10.0);
    assert_eq!(second.origin.y, 15.0);
}

fn first_child(cx: &Cx, root: NodeId) -> NodeId {
    cx.get(root).unwrap().children.first().copied().unwrap()
}

fn layout_rect(cx: &Cx, id: NodeId) -> sigil_hal::Rect {
    cx.get(id)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("layout result exists")
        .rect()
}
