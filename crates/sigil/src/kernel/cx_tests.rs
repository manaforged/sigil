#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use smallvec::smallvec;

use super::*;
use crate::reactive::owner_by_id;

#[test]
fn replace_children_drops_ids_the_kernel_no_longer_holds() {
    let mut cx = Cx::new_root();
    let root = cx.root_owner();
    let mut ids = Vec::new();
    assert!(owner_by_id(root, || {
        for _ in 0..3 {
            ids.push(cx.new_node(NodeType::Container, A11yProps::default()));
        }
    }));
    let [parent, live, dead] = ids.as_slice() else {
        panic!("three nodes mounted");
    };
    let parent = *parent;
    let live = *live;
    let dead = *dead;
    assert!(cx.remove(dead));
    cx.replace_children(parent, smallvec![live, dead]);
    let children = &cx
        .nodes
        .get(parent.key())
        .expect("parent node present")
        .children;
    assert_eq!(
        children.as_slice(),
        &[live],
        "a dead id must not survive into a live parent's child list"
    );
}

#[test]
fn replace_children_reparents_adopted_nodes_without_stale_handles() {
    let mut cx = Cx::new_root();
    let root = cx.root_owner();
    let mut ids = Vec::new();
    assert!(owner_by_id(root, || {
        for _ in 0..3 {
            ids.push(cx.new_node(NodeType::Container, A11yProps::default()));
        }
    }));
    let [a, b, x] = ids.as_slice() else {
        panic!("three nodes mounted");
    };
    let a = *a;
    let b = *b;
    let x = *x;
    cx.replace_children(a, smallvec![x]);
    assert_eq!(
        cx.nodes.get(x.key()).and_then(|node| node.parent),
        Some(a),
        "first listing establishes x under a"
    );
    cx.replace_children(b, smallvec![x]);
    assert_eq!(
        cx.nodes.get(x.key()).and_then(|node| node.parent),
        Some(b),
        "adoption must move x's parent pointer to b"
    );
    assert!(
        cx.nodes
            .get(a.key())
            .expect("a present")
            .children
            .is_empty(),
        "the surrendered child must leave a's child list"
    );
    assert!(
        cx.dirty.contains(&a),
        "the parent that lost a child must be marked dirty"
    );
    assert!(cx.remove(x));
    assert!(
        cx.nodes
            .get(b.key())
            .expect("b present")
            .children
            .is_empty(),
        "remove must detach x from the parent it actually lists"
    );
}
