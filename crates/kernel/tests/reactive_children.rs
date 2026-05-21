#![allow(missing_docs)]
// Integration tests keep assertions direct; unwraps name the expected
// topology and failed assertions are the test output.
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use std::cell::Cell;
use std::rc::Rc;

use accesskit as _;
use ahash as _;
use criterion as _;
use hal as _;
use slotmap as _;
use smallvec as _;

use kernel::{Cx, NodeId, container, text};
use reactive::{Runtime, signal};

fn child_ids(cx: &Cx, parent: NodeId) -> Vec<NodeId> {
    cx.get(parent).expect("parent alive").children().to_vec()
}

#[test]
fn children_reactive_initial_build_adds_children() {
    Runtime::scope("sigil_kernel_children_reactive_initial", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let parent =
            cx.add(container().children_reactive(move || items.get().into_iter().map(text)));

        // Initial build enqueues the first rebuild; explicit flush
        // until B1.b wires it into LayoutEngine::compute.
        cx.flush_reactive_children();

        assert_eq!(child_ids(&cx, parent).len(), 3, "3 initial children");
    });
}

#[test]
fn children_reactive_signal_mutation_replaces_children() {
    Runtime::scope("sigil_kernel_children_reactive_mutate", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let parent =
            cx.add(container().children_reactive(move || items.get().into_iter().map(text)));
        cx.flush_reactive_children();
        let before = child_ids(&cx, parent);
        assert_eq!(before.len(), 3);

        items.set(vec!["x", "y", "z", "w"]);
        cx.flush_reactive_children();

        let after = child_ids(&cx, parent);
        assert_eq!(after.len(), 4, "new children count reflects mutated Vec");
        for old in &before {
            assert!(
                !after.contains(old),
                "positional rebuild: prior id {old:?} must not survive"
            );
            assert!(
                cx.get(*old).is_none(),
                "prior child {old:?} must be removed from arena"
            );
        }
    });
}

#[test]
fn children_reactive_dispose_on_parent_removal() {
    let runs = Rc::new(Cell::new(0_u32));

    Runtime::scope("sigil_kernel_children_reactive_dispose", || {
        let items = signal(vec!["a", "b"]);
        let runs_for_effect = Rc::clone(&runs);
        let mut cx = Cx::new_root();
        let parent = cx.add(container().children_reactive(move || {
            runs_for_effect.set(runs_for_effect.get() + 1);
            items.get().into_iter().map(text).collect::<Vec<_>>()
        }));
        cx.flush_reactive_children();

        assert_eq!(runs.get(), 1, "closure ran on initial build");

        items.set(vec!["x"]);
        cx.flush_reactive_children();
        assert_eq!(runs.get(), 2, "closure ran on first signal mutation");

        assert!(cx.remove(parent), "parent removed");

        items.set(vec!["should-not-rebuild"]);
        cx.flush_reactive_children();
        assert_eq!(
            runs.get(),
            2,
            "owner-tree cascade disposed the effect; closure must not re-run"
        );
    });
}

#[test]
fn children_reactive_empty_vec_removes_all() {
    Runtime::scope("sigil_kernel_children_reactive_empty", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let parent =
            cx.add(container().children_reactive(move || items.get().into_iter().map(text)));
        cx.flush_reactive_children();
        assert_eq!(child_ids(&cx, parent).len(), 3);

        items.set(Vec::new());
        cx.flush_reactive_children();

        assert_eq!(
            child_ids(&cx, parent).len(),
            0,
            "empty Vec drains all children"
        );
    });
}

#[test]
fn children_reactive_reorder_visibly_rebuilds() {
    Runtime::scope("sigil_kernel_children_reactive_reorder", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let parent =
            cx.add(container().children_reactive(move || items.get().into_iter().map(text)));
        cx.flush_reactive_children();
        let before = child_ids(&cx, parent);

        items.set(vec!["c", "b", "a"]);
        cx.flush_reactive_children();
        let after = child_ids(&cx, parent);

        assert_eq!(after.len(), 3, "same count post-reorder");
        // Positional identity: no prior id survives even when the Vec is
        // a permutation of the same values. Identity is purely by index.
        for old in &before {
            assert!(
                !after.contains(old),
                "reorder must not preserve identity: prior id {old:?} reappeared"
            );
        }
    });
}
