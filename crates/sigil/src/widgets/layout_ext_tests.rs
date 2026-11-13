#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::kernel::{Cx, IntoNode, NodeId, Role, container};
use crate::layout::{LayoutStyle, Length};
use crate::reactive::{Owner, signal};
use crate::widgets::Widget;

#[test]
fn empty_overrides_are_pass_through() {
    let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
    let result = LayoutOverrides::new().apply(base.clone());
    assert_eq!(result, base);
}

#[test]
fn width_hint_reads_a_width_set_through_layout() {
    let mut overrides = LayoutOverrides::new();
    overrides.edit(LayoutEdit::Merge(LayoutStyle::default().width(64.0)));
    assert_eq!(overrides.width_hint(), Some(64.0));
}

#[test]
fn width_height_override_replaces_dimensions() {
    let mut overrides = LayoutOverrides::new();
    overrides.edit(LayoutEdit::Width(Length::Px(100.0)));
    overrides.edit(LayoutEdit::Height(Length::Px(50.0)));
    let base = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
    let result = overrides.apply(base);
    let expected = LayoutStyle::flex_row()
        .fixed_size(10.0, 20.0)
        .width(100.0)
        .height(50.0);
    assert_eq!(result, expected);
}

#[test]
fn full_layout_merges_into_base() {
    let mut overrides = LayoutOverrides::new();
    let replacement = LayoutStyle::block().fixed_size(7.0, 9.0);
    overrides.edit(LayoutEdit::Merge(replacement.clone()));
    let mut expected = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
    let result = overrides.apply(expected.clone());
    expected.merge(&replacement);
    assert_eq!(result, expected);
}

#[test]
fn full_layout_then_partials_compose() {
    let mut overrides = LayoutOverrides::new();
    let replacement = LayoutStyle::block().fixed_size(7.0, 9.0);
    overrides.edit(LayoutEdit::Merge(replacement.clone()));
    overrides.edit(LayoutEdit::Width(Length::Px(123.0)));
    let mut expected = LayoutStyle::flex_row().fixed_size(10.0, 20.0);
    let result = overrides.apply(expected.clone());
    expected.merge(&replacement.width(123.0));
    assert_eq!(result, expected);
}

fn assert_sized(cx: &Cx, id: NodeId) {
    let style = cx
        .get(id)
        .expect("node exists")
        .extension::<LayoutStyle>()
        .expect("node has LayoutStyle");
    assert_eq!(style.data().size.width, Length::Px(100.0));
    assert_eq!(style.data().size.height, Length::Px(50.0));
}

fn assert_root_sized<N: Widget + IntoNode>(scope: &'static str, build: impl FnOnce() -> N) {
    Owner::scope(scope, || {
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
    Owner::scope("layout-ext-modal-smoke", || {
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

#[test]
fn path_and_hint_stay_send() {
    fn send_sync<T: Send + Sync + std::panic::UnwindSafe + std::panic::RefUnwindSafe>() {}
    send_sync::<crate::widgets::PathBuilder>();
    send_sync::<crate::widgets::patterns::HintOverlayBuilder>();
}

#[test]
fn cloned_hooks_still_run() {
    let seen = std::rc::Rc::new(std::cell::Cell::new(false));
    let flag = seen.clone();
    let mut hooks = NodeHooks::new();
    hooks.push(Box::new(move |node| {
        flag.set(true);
        node
    }));
    let cloned = hooks.clone();
    let _ = cloned.finish(container());
    assert!(seen.get());
}
