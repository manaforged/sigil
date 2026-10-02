#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::kernel::{Cx, NodeId, NodeType, TextContent, container, node, text};
use sigil::reactive::{Runtime, signal};

fn text_value(cx: &Cx, id: NodeId) -> String {
    match &cx.get(id).expect("node").kind {
        NodeType::Text { content, .. } => content.resolve(),
        other => panic!("expected text, got {other:?}"),
    }
}

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
fn node_erases_mixed_children() {
    Runtime::scope("sigil_kernel_node_mixed", || {
        let items = signal(0_u32);
        let mut cx = Cx::new_root();
        let parent = cx.add(container().children_reactive(move || {
            let _ = items.get();
            [node(text("label")), node(container())]
        }));
        cx.flush_reactive_children();
        assert_eq!(child_ids(&cx, parent).len(), 2);
    });
}

#[test]
fn children_keyed_reorder_keeps_ids() {
    Runtime::scope("sigil_kernel_children_keyed_reorder", || {
        let items = signal(vec!["a", "b", "c"]);
        let mut cx = Cx::new_root();
        let parent = cx.add(
            container()
                .children_keyed(move || items.get().into_iter().map(|label| (label, text(label)))),
        );
        cx.flush_reactive_children();
        let before = child_ids(&cx, parent);
        assert_eq!(before.len(), 3);

        items.set(vec!["c", "b", "a"]);
        cx.flush_reactive_children();
        let after = child_ids(&cx, parent);

        assert_eq!(after, vec![before[2], before[1], before[0]]);
        for id in &before {
            assert!(cx.get(*id).is_some(), "keyed child {id:?} stays live");
        }
    });
}

#[test]
fn children_keyed_drops_missing_and_mounts_new() {
    Runtime::scope("sigil_kernel_children_keyed_diff", || {
        let items = signal(vec!["a", "b"]);
        let mut cx = Cx::new_root();
        let parent = cx.add(
            container()
                .children_keyed(move || items.get().into_iter().map(|label| (label, text(label)))),
        );
        cx.flush_reactive_children();
        let before = child_ids(&cx, parent);
        let first = before[0];

        items.set(vec!["a", "d"]);
        cx.flush_reactive_children();
        let after = child_ids(&cx, parent);

        assert_eq!(after.len(), 2);
        assert_eq!(after[0], first, "key a keeps its node");
        assert!(cx.get(before[1]).is_none(), "key b is removed");
        assert_ne!(after[1], before[1], "key d is a new node");
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
        for old in &before {
            assert!(
                !after.contains(old),
                "reorder must not preserve identity: prior id {old:?} reappeared"
            );
        }
    });
}

#[test]
fn children_reactive_coalesces_rebuilds_before_mounting_captured_state() {
    Runtime::scope("sigil_kernel_children_reactive_coalesce", || {
        let value = signal(0_u32);
        let mut cx = Cx::new_root();
        let parent = cx.add(container().children_reactive(move || {
            let current = value.get();
            let captured = signal(current);
            [move || text(captured.get().to_string())]
        }));
        cx.flush_reactive_children();

        value.set(1);
        value.set(2);
        cx.flush_reactive_children();

        let children = child_ids(&cx, parent);
        let [child] = children.as_slice() else {
            panic!("latest reactive child must be mounted exactly once");
        };
        let rendered = cx.dump_tree(*child);
        assert!(
            rendered.contains('2'),
            "only the latest queued child may mount: {rendered}"
        );
    });
}

#[test]
fn children_keyed_live_text_is_node_owned() {
    Runtime::scope("sigil_kernel_children_keyed_live_text", || {
        let label = signal("alpha".to_string());
        let keys = signal(vec![0_u32]);
        let mut cx = Cx::new_root();
        let parent = cx.add(container().children_keyed(move || {
            keys.get()
                .into_iter()
                .map(move |key| (key, text(move || label.get())))
                .collect::<Vec<_>>()
        }));
        cx.flush_reactive_children();
        let [child] = child_ids(&cx, parent).try_into().expect("one keyed child");
        assert_eq!(text_value(&cx, child), "alpha");
        assert!(
            matches!(
                cx.get(child).expect("child").kind,
                NodeType::Text {
                    content: TextContent::Reactive(_),
                    ..
                }
            ),
            "live text becomes a node-owned signal at build"
        );

        keys.set(vec![0]);
        cx.flush_reactive_children();
        let [kept] = child_ids(&cx, parent)
            .try_into()
            .expect("one keyed child after keep");
        assert_eq!(kept, child, "same key keeps NodeId");

        label.set("omega".to_string());
        assert_eq!(text_value(&cx, kept), "omega");
        assert!(cx.get(kept).is_some(), "node-owned derived stays live");
    });
}

#[test]
fn keyed_rebuild_keeps_mounted_rows_passed_without_a_node() {
    Runtime::scope("sigil_kernel_keyed_rebuild_keep", || {
        let mut cx = Cx::new_root();
        let parent = cx.add(container());
        let prior = Rc::new(std::cell::RefCell::new(Vec::<(u32, NodeId)>::new()));
        sigil::kernel::schedule_keyed_rebuild(
            parent,
            Rc::clone(&prior),
            vec![(1, Some(text("one"))), (2, Some(text("two")))],
        );
        cx.flush_reactive_children();
        let [one, two] = child_ids(&cx, parent).try_into().expect("two rows");
        sigil::kernel::schedule_keyed_rebuild(
            parent,
            Rc::clone(&prior),
            vec![(2, None), (3, Some(text("three"))), (9, None)],
        );
        cx.flush_reactive_children();
        let [kept, three] = child_ids(&cx, parent).try_into().expect("kept and new row");
        assert_eq!(kept, two);
        assert_eq!(text_value(&cx, kept), "two");
        assert_eq!(text_value(&cx, three), "three");
        assert!(cx.get(one).is_none(), "the missing key unmounts");
    });
}
