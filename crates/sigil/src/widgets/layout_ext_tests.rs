#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::kernel::{Cx, IntoNode, NodeId, Role, container};
use crate::layout::{Dimension, LayoutStyle};
use crate::reactive::{Runtime, signal};

#[test]
fn empty_overrides_are_pass_through() {
    let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
    let result = LayoutOverrides::new().apply(base.clone());
    assert_eq!(result, base);
}

#[test]
fn width_height_override_replaces_dimensions() {
    let mut overrides = LayoutOverrides::new();
    overrides.width(100.0);
    overrides.height(50.0);
    let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
    let result = overrides.apply(base);
    let expected = LayoutStyle::flex_row()
        .fixed_size(10.0, 20.0)
        .width(100.0)
        .height(50.0);
    assert_eq!(result, expected);
}

#[test]
fn full_layout_replaces_base() {
    let mut overrides = LayoutOverrides::new();
    let replacement = LayoutStyle::block().fixed_size(7.0, 9.0);
    overrides.layout(replacement.clone());
    let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
    let result = overrides.apply(base);
    assert_eq!(result, replacement);
}

#[test]
fn full_layout_then_partials_compose() {
    let mut overrides = LayoutOverrides::new();
    let replacement = LayoutStyle::block().fixed_size(7.0, 9.0);
    overrides.layout(replacement.clone());
    overrides.width(123.0);
    let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
    let result = overrides.apply(base);
    let expected = replacement.width(123.0);
    assert_eq!(result, expected);
}

fn assert_sized(cx: &Cx, id: NodeId) {
    let style = cx
        .get(id)
        .expect("node exists")
        .get_extension::<LayoutStyle>()
        .expect("node has LayoutStyle");
    assert_eq!(style.data().size.width, Dimension::Px(100.0));
    assert_eq!(style.data().size.height, Dimension::Px(50.0));
}

fn assert_root_sized<N: WidgetLayoutExt + IntoNode>(
    scope: &'static str,
    build: impl FnOnce() -> N,
) {
    Runtime::scope(scope, || {
        let mut cx = Cx::new_root();
        let root = cx.add(build().width(100.0).height(50.0));
        assert_sized(&cx, root);
    });
}

#[test]
fn button_height_override_reaches_root_node() {
    assert_root_sized("layout-ext-button-smoke", || {
        crate::widgets::button::button("OK")
    });
}

#[test]
fn toggle_width_height_override_reaches_root_node() {
    assert_root_sized("layout-ext-toggle-smoke", || {
        crate::widgets::toggle::toggle(signal(false))
    });
}

#[test]
fn checkbox_width_height_override_reaches_root_node() {
    assert_root_sized("layout-ext-checkbox-smoke", || {
        crate::widgets::checkbox::checkbox(signal(false))
    });
}

#[test]
fn text_field_width_height_override_reaches_root_node() {
    assert_root_sized("layout-ext-text-field-smoke", || {
        crate::widgets::text_field::text_field(signal(String::new()))
    });
}

#[test]
fn virtual_list_width_height_override_reaches_root_node() {
    assert_root_sized("layout-ext-virtual-list-smoke", || {
        crate::widgets::virtual_list::virtual_list((0..10).collect::<Vec<u32>>(), |_, _| {
            container()
        })
    });
}

#[test]
fn splitter_width_height_override_reaches_root_node() {
    use crate::widgets::{Orientation, splitter};
    assert_root_sized("layout-ext-splitter-smoke", || {
        splitter(Orientation::Horizontal)
            .child(container(), 200.0)
            .child(container(), 200.0)
    });
}

#[test]
fn tabs_width_height_override_reaches_root_node() {
    assert_root_sized("layout-ext-tabs-smoke", || {
        crate::widgets::tabs::tabs()
            .active(signal(0_usize))
            .labels(vec![String::from("A"), String::from("B")])
    });
}

#[test]
fn modal_width_height_override_reaches_dialog_frame() {
    Runtime::scope("layout-ext-modal-smoke", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            crate::widgets::modal::modal(signal(true))
                .width(100.0)
                .height(50.0),
        );
        let frame = cx
            .get(root)
            .expect("modal root")
            .children()
            .iter()
            .copied()
            .find(|id| cx.get(*id).expect("child").a11y.role == Role::Dialog)
            .expect("dialog frame child");
        assert_sized(&cx, frame);
    });
}
