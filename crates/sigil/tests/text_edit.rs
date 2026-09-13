#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use sigil::kernel::{NodeId, NodeType};
use sigil::layout::prelude::Dimension;
use sigil::prelude::*;
use sigil::testing::with_cx;
use sigil::widgets::{TextEditBuilder, text_edit};

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
fn inner_pane_fills_the_frame() {
    with_cx("text-edit-inner-fill", |cx| {
        let root = cx.add(
            text_edit(signal(String::from("notes")))
                .width(800.0)
                .height(132.0),
        );
        let inner = *cx
            .get(root)
            .expect("root")
            .children()
            .first()
            .expect("inner pane");
        let style = cx
            .get(inner)
            .expect("inner node")
            .get_extension::<LayoutStyle>()
            .expect("inner has LayoutStyle")
            .clone();
        assert_eq!(
            style.data().size.width,
            Dimension::Fill,
            "inner width must follow the frame, not a leftover chrome strip",
        );
        assert_eq!(style.data().size.height, Dimension::Fill);
    });
}

#[test]
fn placeholder_accepts_string_literal() {
    with_cx("text-edit-placeholder", |cx| {
        let value = signal(String::new());
        let _root = cx.add(text_edit(value).placeholder("Type here…"));
    });
}

#[test]
fn text_edit_line_children_rebuild_on_signal_mutation() {
    with_cx("text-edit-lines-rebuild", |cx| {
        let value = signal(String::new());
        let root = cx.add(text_edit(value));
        cx.flush_reactive_children();

        let lines_at_build = count_text_descendants(cx, root);
        assert_eq!(
            lines_at_build, 1,
            "empty buffer should emit one placeholder line child"
        );

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

        value.set(String::from("a"));
        cx.flush_reactive_children();

        assert_eq!(
            count_text_descendants(cx, root),
            1,
            "deleting the newline must collapse to one line child"
        );
    });
}

fn count_text_descendants(cx: &sigil::kernel::Cx, root: NodeId) -> usize {
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
