#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::ignored_unit_patterns
)]

use sigil::a11y::{snapshot, tree};
use sigil::kernel::Role;
use sigil::kernel::{Cx, container};
use sigil::reactive::{Owner, signal};

#[test]
fn disabled_control_stays_visible_and_updates_accessibility() {
    Owner::scope("disabled-control", || {
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let id = cx.add(
            container()
                .role(Role::Button)
                .accessible("Save")
                .focusable(true)
                .mutate(move |node| node.a11y.disabled = Some(disabled)),
        );
        for value in [true, false, true] {
            disabled.set(value);
            let update = tree(&cx, None, &|_| None);
            let (_, node) = update
                .nodes
                .iter()
                .find(|(key, _)| *key == id.to_accesskit())
                .expect("button");
            assert_eq!(node.is_disabled(), value);
            assert!(!node.is_hidden());
            assert_eq!(node.label(), Some("Save"));
        }
    });
}

#[test]
fn checkbox_signal_change_shows_in_tree() {
    Owner::scope("checkbox_signal_change_shows_in_tree", || {
        let checked = signal(false);
        let mut cx = Cx::new_root();
        let _ = cx.add(
            container()
                .role(Role::CheckBox)
                .accessible("Accept terms")
                .checked(checked),
        );

        let first = snapshot(&tree(&cx, None, &|_| None));
        assert!(
            first.contains("Toggled=False"),
            "initial tree shows Toggled=False, got:\n{first}"
        );

        checked.set(true);
        let second = snapshot(&tree(&cx, None, &|_| None));
        assert!(
            second.contains("Toggled=True"),
            "peeked tree shows Toggled=True, got:\n{second}"
        );
        assert!(second.contains("Role=CheckBox"), "got:\n{second}");
        assert!(second.contains("Name=\"Accept terms\""), "got:\n{second}");
    });
}
