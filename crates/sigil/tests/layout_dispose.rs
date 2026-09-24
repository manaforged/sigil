#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]

#[path = "layout_stub/mod.rs"]
mod stub;
use stub::stub_engine;

use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;

#[test]
fn gc_removes_mirrored_nodes_for_disposed_core_subtrees() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(container().child(text("gone"))));
    let removed = cx
        .get(root)
        .and_then(|node| node.children().first().copied())
        .expect("child exists");

    let mut engine = stub_engine();
    engine
        .compute(&mut cx, root, Size::fixed(100.0, 40.0))
        .expect("compute succeeds");
    assert_eq!(engine.mirrored_node_count(), 3);

    assert!(cx.remove(removed));
    engine.gc(&cx).expect("gc succeeds");

    assert_eq!(engine.mirrored_node_count(), 1);
    assert_eq!(engine.result(removed), None);
}
