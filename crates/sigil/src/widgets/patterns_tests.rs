#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use crate::kernel::container as kernel_container;
use crate::reactive::Runtime;

#[test]
fn empty_state_has_title_and_body() {
    Runtime::scope("empty-state", || {
        let mut cx = Cx::new_root();
        let id = empty_state("Summonr", "Message the agent.").build(&mut cx);
        let node = cx.get(id).expect("empty");
        assert_eq!(node.a11y.role, Role::Status);
        assert_eq!(node.children().len(), 2);
    });
}

#[test]
fn card_wraps_one_child() {
    Runtime::scope("card-one-child", || {
        let mut cx = Cx::new_root();
        let id = card(kernel_container()).build(&mut cx);
        let node = cx.get(id).expect("card");
        assert_eq!(node.children().len(), 1);
    });
}

#[test]
fn avatar_is_an_image_role() {
    Runtime::scope("avatar-role", || {
        let mut cx = Cx::new_root();
        let id = avatar("TG").build(&mut cx);
        let node = cx.get(id).expect("avatar");
        assert_eq!(node.a11y.role, Role::Image);
    });
}

#[test]
fn sidebar_is_navigation() {
    Runtime::scope("sidebar-role", || {
        let mut cx = Cx::new_root();
        let id = sidebar(kernel_container()).build(&mut cx);
        let node = cx.get(id).expect("sidebar");
        assert_eq!(node.a11y.role, Role::Navigation);
    });
}

#[test]
fn app_shell_without_modal_has_three_children() {
    Runtime::scope("app-shell-no-modal", || {
        let mut cx = Cx::new_root();
        let id =
            app_shell(container(), container(), container(), None::<NodeBuilder>).build(&mut cx);
        let node = cx.get(id).expect("app-shell root exists");
        assert_eq!(node.a11y.role, Role::GenericContainer);
        assert_eq!(
            node.children().len(),
            3,
            "without modal, app_shell is top + work + status"
        );
    });
}

#[test]
fn app_shell_with_modal_has_four_children() {
    Runtime::scope("app-shell-with-modal", || {
        let mut cx = Cx::new_root();
        let id = app_shell(container(), container(), container(), Some(container())).build(&mut cx);
        let node = cx.get(id).expect("app-shell root exists");
        assert_eq!(
            node.children().len(),
            4,
            "with modal, app_shell appends the overlay as a 4th sibling"
        );
    });
}

#[test]
fn field_holds_label_and_control_in_order() {
    Runtime::scope("field-static-label", || {
        let mut cx = Cx::new_root();
        let id = field("Username", container()).build(&mut cx);
        let node = cx.get(id).expect("field root exists");
        assert_eq!(node.a11y.role, Role::GenericContainer);
        assert_eq!(
            node.children().len(),
            2,
            "field produces exactly label + control"
        );
    });
}

#[test]
fn a_field_control_is_named_by_its_visible_label() {
    use crate::widgets::text_field::text_field;
    Runtime::scope("field-label-wins", || {
        let mut cx = Cx::new_root();
        let id = field(
            "Full name",
            text_field(crate::reactive::signal(String::new())),
        )
        .build(&mut cx);
        let [label_id, control_id] = *cx.get(id).expect("field root").children() else {
            panic!("field produces label and control");
        };
        let control = cx.get(control_id).expect("control");
        assert_eq!(control.a11y.labelled_by, Some(label_id));
        assert!(
            matches!(control.a11y.name, AccessibleName::None),
            "the control must not keep its Text field fallback name"
        );
        let label = crate::a11y::mapping::node(&cx, label_id, &|_| None);
        assert_eq!(label.value(), Some("Full name"));
    });
}

#[test]
fn validated_field_emits_three_children() {
    use crate::widgets::text_field::text_field;
    Runtime::scope("validated-field-shape", || {
        let mut cx = Cx::new_root();
        let error: crate::reactive::Signal<Option<String>> = crate::reactive::signal(None);
        let value = crate::reactive::signal(String::new());
        let id = validated_field("Email", text_field(value), error).build(&mut cx);
        let node = cx.get(id).expect("validated field root");
        assert_eq!(
            node.children().len(),
            3,
            "validated_field is label + control + error-slot",
        );
    });
}

#[test]
fn kv_grid_emits_one_row_per_input() {
    Runtime::scope("kv-grid-three-rows", || {
        let mut cx = Cx::new_root();
        let id = kv_grid(&[
            ("Content-Type", "application/json"),
            ("Status", "200 OK"),
            ("Length", "1024"),
        ])
        .build(&mut cx);
        let node = cx.get(id).expect("kv-grid root exists");
        assert_eq!(
            node.children().len(),
            3,
            "kv_grid produces one row child per input tuple"
        );
    });
}

#[test]
fn kv_grid_empty_input_has_no_rows() {
    Runtime::scope("kv-grid-empty", || {
        let mut cx = Cx::new_root();
        let id = kv_grid(&[]).build(&mut cx);
        let node = cx.get(id).expect("kv-grid root exists");
        assert!(
            node.children().is_empty(),
            "empty input must produce zero rows"
        );
    });
}

#[test]
fn patterns_tooltip_wraps_target() {
    Runtime::scope("patterns-tooltip", || {
        let _builder = tooltip(kernel_container(), "Save the file");
    });
}

#[test]
fn hint_overlay_records_entries() {
    let overlay = hint_overlay(&[("Ctrl+S", "Save"), ("?", "Help")]).entry("Esc", "Close");
    assert_eq!(overlay.entries.len(), 3);
    assert_eq!(overlay.entries[0].0, "Ctrl+S");
    assert_eq!(overlay.entries[2].1, "Close");
}

#[test]
fn hint_overlay_builds() {
    Runtime::scope("hint-overlay-build", || {
        let _node = hint_overlay(&[("Ctrl+S", "Save")]).into_node_builder();
    });
}
