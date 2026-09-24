#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

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
fn placeholder_accepts_string_literal() {
    with_cx("text-edit-placeholder", |cx| {
        let value = signal(String::new());
        let _root = cx.add(text_edit(value).placeholder("Type here…"));
    });
}
