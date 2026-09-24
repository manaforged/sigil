#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use ahash::AHashSet;
use sigil::kernel::{Cx, NodeId, container, text};

fn dirty_set(cx: &Cx) -> AHashSet<NodeId> {
    cx.dirty().collect()
}

#[test]
fn dirty_set_starts_empty() {
    let cx = Cx::new_root();

    assert!(cx.dirty().next().is_none());
}

#[test]
fn add_dirties_new_node() {
    let mut cx = Cx::new_root();
    let id = cx.add(text("new"));

    assert!(dirty_set(&cx).contains(&id));
}

#[test]
fn add_child_dirties_parent_and_child() {
    let mut cx = Cx::new_root();
    let parent = cx.add(container());
    cx.clear_dirty();

    let child = cx.add_child(parent, text("child"));
    let dirty = dirty_set(&cx);

    assert!(dirty.contains(&parent));
    assert!(dirty.contains(&child));
}

#[test]
fn remove_dirties_parent_and_clears_self() {
    let mut cx = Cx::new_root();
    let parent = cx.add(container());
    let child = cx.add_child(parent, text("child"));
    cx.clear_dirty();
    let _ = cx.get_mut(child).expect("child exists");

    assert!(cx.remove(child));
    let dirty = dirty_set(&cx);

    assert!(dirty.contains(&parent));
    assert!(!dirty.contains(&child));
}

#[test]
fn get_mut_dirties() {
    let mut cx = Cx::new_root();
    let id = cx.add(text("mutated"));
    cx.clear_dirty();

    let _ = cx.get_mut(id).expect("node exists");

    assert!(dirty_set(&cx).contains(&id));
}

#[test]
fn clear_dirty_empties_set() {
    let mut cx = Cx::new_root();
    let _ = cx.add(text("dirty"));

    cx.clear_dirty();

    assert!(cx.dirty().next().is_none());
}

#[test]
fn mark_mutated_dirties_everything() {
    let mut cx = Cx::new_root();
    let _ = cx.add(
        container()
            .child(text("first"))
            .child(container().child(text("nested"))),
    );
    cx.clear_dirty();

    cx.mark_mutated();

    assert_eq!(cx.dirty().count(), cx.len());
}
