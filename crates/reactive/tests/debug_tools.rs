#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic)]
#![allow(clippy::indexing_slicing)]

use sigil_reactive::debug::{dump_owner_tree, dump_subtree};
use sigil_reactive::{Owner, Runtime, effect, on_cleanup, signal};

#[test]
fn dump_owner_tree_renders_single_root() {
    let dump = Runtime::scope("single_root", dump_owner_tree);

    let lines: Vec<&str> = dump.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "exactly one owner should appear; got:\n{dump}"
    );
    assert!(
        lines[0].starts_with("single_root  at "),
        "line should begin with the root owner name; got {lines:?}"
    );
    assert!(
        !lines[0].contains('['),
        "empty root should have no count bracket; got {lines:?}"
    );
    assert!(dump.ends_with('\n'), "dump must end with a newline");
}

#[test]
fn dump_owner_tree_renders_nested_owners_with_names_and_locations() {
    let dump = Runtime::scope("nested_root", || {
        let outer = Owner::new("Window(main)", Owner::current());
        outer.with(|| {
            let _inner_a = Owner::new("Button(a)", Owner::current());
            let _inner_b = Owner::new("Button(b)", Owner::current());
        });
        dump_owner_tree()
    });

    let lines: Vec<&str> = dump.lines().collect();
    assert_eq!(lines.len(), 4, "expected 4 lines, got:\n{dump}");

    assert!(lines[0].starts_with("nested_root  at "));
    assert!(lines[1].starts_with("  Window(main)  at "));
    assert!(lines[2].starts_with("    Button(a)  at "));
    assert!(lines[3].starts_with("    Button(b)  at "));

    assert!(
        lines[1].contains("debug_tools.rs"),
        "Window(main) should report this test file; got {:?}",
        lines[1]
    );
    assert!(
        lines[2].contains("debug_tools.rs"),
        "Button(a) should report this test file; got {:?}",
        lines[2]
    );
}

#[test]
fn dump_owner_tree_includes_signal_and_effect_counts() {
    let dump = Runtime::scope("counts_root", || {
        let leaf = Owner::new("Leaf", Owner::current());
        leaf.with(|| {
            let _a = signal(0_i32);
            let _b = signal(0_i32);
            on_cleanup(|| {});
        });
        dump_owner_tree()
    });

    let leaf_line = dump
        .lines()
        .find(|l| l.trim_start().starts_with("Leaf"))
        .unwrap_or_else(|| panic!("no `Leaf` line in dump:\n{dump}"));

    assert!(
        leaf_line.contains("signals: 2"),
        "Leaf line should count 2 signals; got {leaf_line:?}"
    );
    assert!(
        leaf_line.contains("cleanups: 1"),
        "Leaf line should count 1 cleanup; got {leaf_line:?}"
    );
    assert!(
        leaf_line.contains("[signals: 2, cleanups: 1]"),
        "counts should share one bracket; got {leaf_line:?}"
    );
}

#[test]
fn dump_owner_tree_renders_effect_run_owner_as_child() {
    let dump = Runtime::scope("with_effect", || {
        let host = Owner::new("Host", Owner::current());
        host.with(|| {
            let s = signal(0_i32);
            effect(move || {
                let _ = s.get();
            });
        });
        dump_owner_tree()
    });

    assert!(
        dump.contains("<effect-run>"),
        "effect run-owner should appear as a named child; got:\n{dump}"
    );

    let host_line = dump
        .lines()
        .find(|l| l.trim_start().starts_with("Host"))
        .unwrap_or_else(|| panic!("no `Host` line in dump:\n{dump}"));
    assert!(
        host_line.contains("effects: 1"),
        "Host line should count 1 effect; got {host_line:?}"
    );
}

#[test]
fn dump_subtree_scopes_to_given_owner() {
    Runtime::scope("subtree_root", || {
        let sibling_a = Owner::new("SiblingA", Owner::current());
        sibling_a.with(|| {
            let _a = signal(1_i32);
        });
        let sibling_b = Owner::new("SiblingB", Owner::current());
        sibling_b.with(|| {
            let _leaf = Owner::new("Leaf", Owner::current());
        });

        let full = dump_owner_tree();
        let sub = dump_subtree(sibling_b.id());

        assert!(
            full.contains("SiblingA"),
            "full dump should contain SiblingA; got:\n{full}"
        );
        assert!(
            full.contains("SiblingB"),
            "full dump should contain SiblingB; got:\n{full}"
        );

        assert!(
            !sub.contains("SiblingA"),
            "subtree of B should not contain A; got:\n{sub}"
        );
        assert!(
            !sub.contains("subtree_root"),
            "subtree of B should not contain the root; got:\n{sub}"
        );
        let lines: Vec<&str> = sub.lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 lines in subtree, got:\n{sub}");
        assert!(
            lines[0].starts_with("SiblingB  at "),
            "subtree root should be at depth zero; got {:?}",
            lines[0]
        );
        assert!(
            lines[1].starts_with("  Leaf  at "),
            "descendant should be indented 2 spaces; got {:?}",
            lines[1]
        );
    });
}

#[test]
fn dump_owner_tree_is_empty_when_no_owners_exist() {
    let dump = dump_owner_tree();
    assert_eq!(
        dump, "",
        "dump with no live owners should be empty; got {dump:?}"
    );
}
