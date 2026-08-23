use sigil_kernel::{Cx, IntoNode, NodeBuilder, container};
use sigil_reactive::Runtime;

use super::{SplitterOrientation, splitter};

fn two_pane() -> impl IntoNode {
    splitter(SplitterOrientation::Horizontal)
        .child(container(), 120.0)
        .child(container(), 200.0)
}

#[test]
fn helper_returns_into_node() {
    Runtime::scope("splitter-into", || {
        let mut cx = Cx::new_root();
        let id = cx.add(two_pane());
        assert!(cx.get(id).is_some());
    });
}

#[test]
fn splitter_converts_to_node_builder() {
    Runtime::scope("splitter-from", || {
        let node: NodeBuilder = splitter(SplitterOrientation::Horizontal)
            .child(container(), 120.0)
            .child(container(), 200.0)
            .into();
        let mut cx = Cx::new_root();
        let id = cx.add(node);
        assert!(cx.get(id).is_some());
    });
}

#[test]
fn pane_builder_converts_to_node_builder() {
    Runtime::scope("pane-from", || {
        let node: NodeBuilder = splitter(SplitterOrientation::Horizontal)
            .pane(container())
            .initial(120.0)
            .pane(container())
            .grow(1.0)
            .into();
        let mut cx = Cx::new_root();
        let id = cx.add(node);
        assert!(cx.get(id).is_some());
    });
}
