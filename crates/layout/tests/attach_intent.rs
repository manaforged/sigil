#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]

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

#[test]
fn sync_updates_intent_without_allocating_new_layout_nodes() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(text("A")).child(text("B")));

    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_row().fixed_size(100.0, 20.0));

    let mut engine = stub_engine();
    engine.sync(&cx, root).expect("initial sync succeeds");
    assert_eq!(engine.mirrored_node_count(), 3);

    cx.get_mut(root)
        .expect("root exists")
        .attach(LayoutStyle::flex_row().fixed_size(200.0, 20.0));
    engine.sync(&cx, root).expect("resync succeeds");

    assert_eq!(engine.mirrored_node_count(), 3);
    engine
        .compute(&mut cx, root, Size::fixed(200.0, 20.0))
        .expect("compute succeeds");

    let result = cx
        .get(root)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("root result attached");
    assert_eq!(result.rect().width, 200.0);
}

#[test]
fn text_builder_accepts_layout_intent() {
    let mut cx = Cx::new_root();
    let root = cx.add(text("Pinned").layout(LayoutStyle::block().width(96.0)));

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(320.0, 240.0))
        .expect("compute succeeds");

    let result = cx
        .get(root)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .expect("text result attached");
    assert_eq!(result.rect().width, 96.0);
}
