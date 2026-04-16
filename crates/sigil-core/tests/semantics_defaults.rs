#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use accesskit as _;
use ahash as _;
use criterion as _;
use sigil_reactive as _;
use slotmap as _;
use smallvec as _;

use sigil_core::{Cx, Role, container, fragment, text};

#[test]
fn every_core_node_kind_has_accessible_defaults() {
    let mut cx = Cx::new_root();
    let container = cx.add(container());
    let fragment = cx.add(fragment());
    let text = cx.add(text("Label text"));

    for id in [container, fragment, text] {
        let semantics = &cx.get(id).expect("node exists").semantics;
        assert_ne!(semantics.role, Role::Unknown);
        assert!(semantics.aria_owns.is_empty());
        assert_eq!(semantics.aria_labelled_by, None);
    }

    assert_eq!(
        cx.get(container).expect("container").semantics.role,
        Role::GenericContainer
    );
    assert_eq!(
        cx.get(fragment).expect("fragment").semantics.role,
        Role::GenericContainer
    );
    assert_eq!(cx.get(text).expect("text").semantics.role, Role::Label);
}
