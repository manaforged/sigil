//! Integration tests for the public `text_edit` widget surface.
//!
//! Covers the factory + builder API: factory shape, change-callback
//! delivery, layout-override pass-through, and headless tree-build.
//! The internal `TextEdit` rope primitive is covered by `#[cfg(test)]`
//! tests inside `crates/sigil/src/widgets/text_edit/buffer.rs`.

#![allow(missing_docs)]

use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use arboard as _;
use criterion as _;
use events as _;
use hal as _;
use image as _;
use kernel::{NodeId, NodeType};
use layout::prelude::Dimension;
use pollster as _;
use reactive as _;
use renderer as _;
use sigil::prelude::*;
use sigil::testing::with_cx;
use sigil::widgets::{TextEditBuilder, text_edit};
use style as _;
use tasks as _;
use wgpu as _;

#[test]
fn tree_builds_without_panic() {
    with_cx("text-edit-tree", |cx| {
        let value = signal(String::new());
        let _root = cx.add(
            text_edit(value)
                .placeholder("Notes…")
                .width(320.0)
                .height(120.0),
        );
    });
}

#[test]
fn factory_returns_a_builder_that_carries_supplied_signal() {
    with_cx("text-edit-factory-binds-signal", |cx| {
        // The builder threads the supplied Signal<String> through into
        // the rendered node's a11y text_value, so we can assert
        // post-build that the same signal reached the leaf.
        let value = signal(String::from("hello"));
        let builder: TextEditBuilder = text_edit(value);
        let root = cx.add(builder);
        let bound = cx
            .get(root)
            .expect("root node exists")
            .a11y
            .text_value
            .expect("text_value populated");
        assert_eq!(bound.peek(), "hello");
    });
}

#[test]
fn submit_callback_is_registered_without_firing_on_construction() {
    // Submit-on-Ctrl+Enter is wired through a key handler that we
    // cannot dispatch synthetically without a winit-driven event
    // loop. As a smoke, assert the callback is accepted and is not
    // invoked at builder-construction time.
    use std::cell::Cell;
    use std::rc::Rc;
    with_cx("text-edit-submit-not-fired", |cx| {
        let value = signal(String::from("hello"));
        let fired = Rc::new(Cell::new(false));
        let fired_for_cb = fired.clone();
        let _root = cx.add(text_edit(value).submit(move |_text: &str| {
            fired_for_cb.set(true);
        }));
        assert!(!fired.get(), "submit must not fire at construction");
    });
}

#[test]
fn widget_layout_ext_overrides_reach_root_node() {
    with_cx("text-edit-layout-override", |cx| {
        let value = signal(String::new());
        let root = cx.add(text_edit(value).width(200.0).height(150.0));
        let style = cx
            .get(root)
            .expect("root node exists")
            .get_extension::<LayoutStyle>()
            .expect("root has LayoutStyle")
            .clone();
        assert_eq!(style.data().size.width, Dimension::Px(200.0));
        assert_eq!(style.data().size.height, Dimension::Px(150.0));
    });
}

#[test]
fn placeholder_accepts_string_literal() {
    with_cx("text-edit-placeholder", |cx| {
        let value = signal(String::new());
        let _root = cx.add(text_edit(value).placeholder("Type here…"));
    });
}

/// B4 — line children rebuild reactively when the bound signal mutates.
///
/// Was previously a "pins the broken behaviour" regression test
/// (`text_edit_line_children_freeze_at_build`). After B1
/// (`children_reactive`) + B4 wired `edit_lines` through it, the same
/// scenario now produces the correct count, so the assertion flipped
/// from `lines_after_mutation == lines_at_build` to
/// `lines_after_mutation == 2` (one node per line in `"a\nb"`).
#[test]
fn text_edit_line_children_rebuild_on_signal_mutation() {
    with_cx("text-edit-lines-rebuild", |cx| {
        let value = signal(String::new());
        let root = cx.add(text_edit(value));
        cx.flush_reactive_children();

        let lines_at_build = count_text_descendants(cx, root);
        // Empty buffer: single empty-line placeholder text child.
        assert_eq!(
            lines_at_build, 1,
            "empty buffer should emit one placeholder line child"
        );

        // Mutate the bound signal AFTER build — line children rebuild
        // against the fresh line vector via `children_reactive`.
        value.set(String::from("a\nb"));
        cx.flush_reactive_children();
        let lines_after_mutation = count_text_descendants(cx, root);
        assert_eq!(
            lines_after_mutation, 2,
            "two newline-separated lines should emit two text children"
        );
        assert_eq!(value.peek(), "a\nb");
    });
}

/// Typing three lines after build expands the child set to three.
#[test]
fn text_edit_line_children_grow_to_three_lines() {
    with_cx("text-edit-lines-grow", |cx| {
        let value = signal(String::new());
        let root = cx.add(text_edit(value));
        cx.flush_reactive_children();

        value.set(String::from("one\ntwo\nthree"));
        cx.flush_reactive_children();

        assert_eq!(
            count_text_descendants(cx, root),
            3,
            "three newline-separated lines should emit three text children"
        );
    });
}

/// Deleting the trailing newline collapses two visible lines to one.
#[test]
fn text_edit_line_children_collapse_after_delete() {
    with_cx("text-edit-lines-collapse", |cx| {
        let value = signal(String::from("a\nb"));
        let root = cx.add(text_edit(value));
        cx.flush_reactive_children();

        assert_eq!(
            count_text_descendants(cx, root),
            2,
            "initial buffer of two lines should render two children"
        );

        // Simulate the user backspacing the second line away.
        value.set(String::from("a"));
        cx.flush_reactive_children();

        assert_eq!(
            count_text_descendants(cx, root),
            1,
            "deleting the newline must collapse to one line child"
        );
    });
}

/// Walk the subtree rooted at `root` and count nodes whose kind is
/// [`NodeType::Text`]. Fragments are descended through; containers
/// are also descended. Used by the freeze-regression test.
fn count_text_descendants(cx: &kernel::Cx, root: NodeId) -> usize {
    let mut stack = vec![root];
    let mut count = 0_usize;
    while let Some(id) = stack.pop() {
        let Some(node) = cx.get(id) else { continue };
        if matches!(node.kind, NodeType::Text { .. }) {
            count += 1;
        }
        stack.extend(node.children().iter().copied());
    }
    count
}
