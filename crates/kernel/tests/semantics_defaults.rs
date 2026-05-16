#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use accesskit as _;
use ahash as _;
use criterion as _;
use sigil_hal as _;
use sigil_reactive::{Runtime, signal};
use slotmap as _;
use smallvec as _;

use sigil_kernel::{Cx, Role, container, fragment, text};

#[test]
fn every_core_node_kind_has_accessible_defaults() {
    let mut cx = Cx::new_root();
    let container = cx.add(container());
    let fragment = cx.add(fragment());
    let text = cx.add(text("Label text"));

    for id in [container, fragment, text] {
        let a11y = &cx.get(id).expect("node exists").a11y;
        assert_ne!(a11y.role, Role::Unknown);
        assert!(a11y.aria_owns.is_empty());
        assert_eq!(a11y.aria_labelled_by, None);
    }

    assert_eq!(
        cx.get(container).expect("container").a11y.role,
        Role::GenericContainer
    );
    assert_eq!(
        cx.get(fragment).expect("fragment").a11y.role,
        Role::GenericContainer
    );
    assert_eq!(cx.get(text).expect("text").a11y.role, Role::Label);
}

#[test]
fn text_input_semantics_carry_reactive_value_and_placeholder() {
    Runtime::scope("sigil-kernel-text-input-semantics", || {
        let value = signal(String::from("draft"));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .role(Role::TextInput)
                .aria_text_value_signal(value)
                .aria_placeholder("Notes"),
        );

        let a11y = &cx.get(root).expect("text input node exists").a11y;
        assert_eq!(a11y.role, Role::TextInput);
        assert_eq!(a11y.text_value.expect("text value").get(), "draft");
        assert_eq!(a11y.placeholder.as_deref(), Some("Notes"));

        value.set(String::from("submitted"));
        assert_eq!(a11y.text_value.expect("text value").get(), "submitted");
    });
}
