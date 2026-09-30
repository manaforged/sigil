#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::layout::Dimension;
use crate::layout::LayoutStyle;

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

fn root_layout_style(cx: &crate::kernel::Cx, root: crate::kernel::NodeId) -> LayoutStyle {
    cx.get(root)
        .expect("root node exists")
        .get_extension::<LayoutStyle>()
        .expect("root has LayoutStyle")
        .clone()
}

#[test]
fn button_height_override_reaches_root_node() {
    use crate::widgets::button::button;
    crate::reactive::Runtime::scope("layout-ext-button-smoke", || {
        let mut cx = crate::kernel::Cx::new_root();
        let root = cx.add(button("OK").width(100.0).height(50.0));
        let style = root_layout_style(&cx, root);
        assert_eq!(style.data().size.width, Dimension::Px(100.0));
        assert_eq!(style.data().size.height, Dimension::Px(50.0));
    });
}

#[test]
fn toggle_width_height_override_reaches_root_node() {
    use crate::widgets::toggle::toggle;
    crate::reactive::Runtime::scope("layout-ext-toggle-smoke", || {
        let mut cx = crate::kernel::Cx::new_root();
        let on = crate::reactive::signal(false);
        let root = cx.add(toggle(on).width(100.0).height(50.0));
        let style = root_layout_style(&cx, root);
        assert_eq!(style.data().size.width, Dimension::Px(100.0));
        assert_eq!(style.data().size.height, Dimension::Px(50.0));
    });
}

#[test]
fn checkbox_width_height_override_reaches_root_node() {
    use crate::widgets::checkbox::checkbox;
    crate::reactive::Runtime::scope("layout-ext-checkbox-smoke", || {
        let mut cx = crate::kernel::Cx::new_root();
        let checked = crate::reactive::signal(false);
        let root = cx.add(checkbox(checked).width(100.0).height(50.0));
        let style = root_layout_style(&cx, root);
        assert_eq!(style.data().size.width, Dimension::Px(100.0));
        assert_eq!(style.data().size.height, Dimension::Px(50.0));
    });
}

#[test]
fn text_field_width_height_override_reaches_root_node() {
    use crate::widgets::text_field::text_field;
    crate::reactive::Runtime::scope("layout-ext-text-field-smoke", || {
        let mut cx = crate::kernel::Cx::new_root();
        let value = crate::reactive::signal(String::new());
        let root = cx.add(text_field(value).width(100.0).height(50.0));
        let style = root_layout_style(&cx, root);
        assert_eq!(style.data().size.width, Dimension::Px(100.0));
        assert_eq!(style.data().size.height, Dimension::Px(50.0));
    });
}

#[test]
fn virtual_list_width_height_override_reaches_root_node() {
    use crate::widgets::virtual_list::virtual_list;
    crate::reactive::Runtime::scope("layout-ext-virtual-list-smoke", || {
        let mut cx = crate::kernel::Cx::new_root();
        let items: Vec<u32> = (0..10).collect();
        let root = cx.add(
            virtual_list(items, |_, _| crate::kernel::container())
                .width(100.0)
                .height(50.0),
        );
        let style = root_layout_style(&cx, root);
        assert_eq!(style.data().size.width, Dimension::Px(100.0));
        assert_eq!(style.data().size.height, Dimension::Px(50.0));
    });
}

#[test]
fn splitter_width_height_override_reaches_root_node() {
    use crate::widgets::splitter::{SplitterOrientation, splitter};
    crate::reactive::Runtime::scope("layout-ext-splitter-smoke", || {
        let mut cx = crate::kernel::Cx::new_root();
        let root = cx.add(
            splitter(SplitterOrientation::Horizontal)
                .child(crate::kernel::container(), 200.0)
                .child(crate::kernel::container(), 200.0)
                .width(100.0)
                .height(50.0),
        );
        let style = root_layout_style(&cx, root);
        assert_eq!(style.data().size.width, Dimension::Px(100.0));
        assert_eq!(style.data().size.height, Dimension::Px(50.0));
    });
}

#[test]
fn tabs_width_height_override_reaches_root_node() {
    use crate::widgets::tabs::tabs;
    crate::reactive::Runtime::scope("layout-ext-tabs-smoke", || {
        let mut cx = crate::kernel::Cx::new_root();
        let active = crate::reactive::signal(0_usize);
        let labels = vec![String::from("A"), String::from("B")];
        let root = cx.add(
            tabs()
                .active(active)
                .labels(labels)
                .width(100.0)
                .height(50.0),
        );
        let style = root_layout_style(&cx, root);
        assert_eq!(style.data().size.width, Dimension::Px(100.0));
        assert_eq!(style.data().size.height, Dimension::Px(50.0));
    });
}

#[test]
fn modal_width_height_override_reaches_dialog_frame() {
    use crate::widgets::modal::modal;
    crate::reactive::Runtime::scope("layout-ext-modal-smoke", || {
        let mut cx = crate::kernel::Cx::new_root();
        let show = crate::reactive::signal(true);
        let root = cx.add(modal(show).width(100.0).height(50.0));
        let frame = cx
            .get(root)
            .expect("modal root")
            .children()
            .iter()
            .copied()
            .find(|id| {
                matches!(
                    cx.get(*id).expect("child").a11y.role,
                    crate::kernel::Role::Dialog
                )
            })
            .expect("dialog frame child");
        let style = cx
            .get(frame)
            .expect("frame node")
            .get_extension::<LayoutStyle>()
            .expect("frame has LayoutStyle")
            .clone();
        assert_eq!(style.data().size.width, Dimension::Px(100.0));
        assert_eq!(style.data().size.height, Dimension::Px(50.0));
    });
}
