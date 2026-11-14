#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use crate::kernel::AccessibleName;
use crate::kernel::Cx;
use crate::kernel::container as kernel_container;
use crate::reactive::Owner;
use crate::widgets::field;

fn shape<N: IntoNode>(name: &'static str, make: impl FnOnce() -> N) -> (Role, usize) {
    Owner::scope(name, || {
        let mut cx = Cx::new_root();
        let id = make().build(&mut cx);
        let node = cx.get(id).expect(name);
        (node.a11y.role, node.children().len())
    })
}

#[test]
fn pattern_roots_have_their_role_and_children() {
    assert_eq!(
        shape("empty-state", || empty_state("No messages")
            .description("Send a message to start.")),
        (Role::Status, 2)
    );
    assert_eq!(shape("card", || card(kernel_container())).1, 1);
    assert_eq!(shape("avatar", || avatar("TG")).0, Role::Image);
    assert_eq!(
        shape("sidebar", || sidebar(kernel_container())).0,
        Role::Navigation
    );
    assert_eq!(
        shape("hint-overlay", || hint_overlay(&[("Ctrl+S", "Save")])),
        (Role::GenericContainer, 1)
    );
}

#[test]
fn app_shell_appends_the_modal_as_a_fourth_child() {
    let none = || app_shell(container(), container(), container(), None::<NodeBuilder>);
    assert_eq!(shape("app-shell", none), (Role::GenericContainer, 3));
    let some = || app_shell(container(), container(), container(), Some(container()));
    assert_eq!(shape("app-shell-modal", some).1, 4);
}

#[test]
fn field_holds_label_control_and_error_slot() {
    use crate::widgets::text_field::text_field;
    assert_eq!(
        shape("field", || field("Username", container())),
        (Role::GenericContainer, 2)
    );
    let validated = || {
        let error: crate::reactive::Signal<Option<String>> = crate::reactive::signal(None);
        field("Email", text_field(crate::reactive::signal(String::new()))).error(error)
    };
    assert_eq!(shape("validated-field", validated).1, 3);
}

#[test]
fn a_field_control_is_named_by_its_visible_label() {
    use crate::widgets::text_field::text_field;
    Owner::scope("field-label-wins", || {
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
fn kv_grid_emits_one_row_per_input() {
    let rows = [
        ("Content-Type", "application/json"),
        ("Status", "200 OK"),
        ("Length", "1024"),
    ];
    assert_eq!(shape("kv-grid", || kv_grid(rows)).1, 3);
    assert_eq!(
        shape("kv-grid-empty", || kv_grid(Vec::<(&str, &str)>::new())).1,
        0
    );
}

#[test]
fn hint_overlay_records_entries() {
    let overlay = hint_overlay(&[("Ctrl+S", "Save"), ("?", "Help")]).entry("Esc", "Close");
    assert_eq!(overlay.entries.len(), 3);
    assert_eq!(overlay.entries[0].0, "Ctrl+S");
    assert_eq!(overlay.entries[2].1, "Close");
}

#[test]
fn app_shell_modal_wrapper_is_not_hittable_and_field_labels_color_input() {
    use crate::style::Color;
    use crate::widgets::color_field::color_field;
    Owner::scope("app-shell-modal-and-color-field", || {
        let mut cx = Cx::new_root();
        let shell_id =
            app_shell(container(), container(), container(), Some(container())).build(&mut cx);
        let modal_wrap = cx.get(shell_id).expect("shell").children()[3];
        assert!(!cx.get(modal_wrap).expect("modal wrap").a11y.hittable);

        let field_id = field(
            "Accent Color",
            color_field(crate::reactive::signal(Color::rgb(0, 0, 0))),
        )
        .build(&mut cx);
        let [label_id, control_id] = *cx.get(field_id).expect("field").children() else {
            panic!("field produces label and control");
        };
        let input_id = cx.get(control_id).expect("color row").children()[1];
        let input = cx.get(input_id).expect("inner text input");
        assert_eq!(input.a11y.role, Role::TextInput);
        assert_eq!(input.a11y.labelled_by, Some(label_id));
        assert!(matches!(input.a11y.name, AccessibleName::None));
    });
}

#[test]
fn interactive_chip_is_named_and_field_labels_nested_editable_markdown() {
    use crate::style::Tone;
    use crate::widgets::chip::chip;
    use crate::widgets::markdown_edit;
    Owner::scope("chip-name-and-nested-markdown-field", || {
        let mut cx = Cx::new_root();
        let chip_id = chip("Filter", Tone::Info).activate(|| {}).build(&mut cx);
        let chip_node = cx.get(chip_id).expect("chip");
        assert_eq!(chip_node.a11y.role, Role::Button);
        assert!(matches!(
            &chip_node.a11y.name,
            AccessibleName::Static(name) if name == "Filter"
        ));

        let field_id = field(
            "Markdown",
            markdown_edit(crate::reactive::signal(String::new())),
        )
        .build(&mut cx);
        let [label_id, control_id] = *cx.get(field_id).expect("field").children() else {
            panic!("field produces label and control");
        };
        let mut stack = vec![control_id];
        let mut found = None;
        while let Some(id) = stack.pop() {
            let node = cx.get(id).expect("node");
            if node.a11y.role == Role::TextInput {
                found = Some(node);
                break;
            }
            stack.extend(node.children().iter().copied());
        }
        let input = found.expect("nested markdown TextInput");
        assert_eq!(input.a11y.labelled_by, Some(label_id));
        assert!(matches!(input.a11y.name, AccessibleName::None));
    });
}
