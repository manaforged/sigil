#![allow(missing_docs)]
// Integration tests keep assertions direct; unwraps name the expected
// topology and failed assertions are the test output.
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use accesskit as _;
use ahash as _;
use criterion as _;
use hal as _;
use reactive as _;
use slotmap as _;
use smallvec as _;

use kernel::{Cx, IntoNode, NodeId, Role, container, text};

struct LabelledPanel {
    title: &'static str,
    body: &'static str,
}

impl IntoNode for LabelledPanel {
    fn build(self, cx: &mut Cx) -> NodeId {
        let root = cx.add(container().role(Role::Group));
        let _ = cx.add_child(root, text(self.title).role(Role::Heading));
        let _ = cx.add_child(root, text(self.body));
        root
    }
}

#[test]
fn add_child_links_parent_and_uses_component_owner() {
    let mut cx = Cx::new_root();
    let panel = cx.add(LabelledPanel {
        title: "Title",
        body: "Body",
    });

    let panel_node = cx.get(panel).expect("panel exists");
    assert!(matches!(panel_node.a11y.role, Role::Group));
    assert_eq!(panel_node.children().len(), 2);

    let heading = panel_node.children()[0];
    let body = panel_node.children()[1];

    assert_eq!(cx.get(heading).expect("heading").parent(), Some(panel));
    assert_eq!(cx.get(body).expect("body").parent(), Some(panel));

    // Each add_child created its own component owner distinct from the
    // panel's owner.
    let panel_owner = cx.component_owner(panel).expect("panel owner");
    let heading_owner = cx.component_owner(heading).expect("heading owner");
    let body_owner = cx.component_owner(body).expect("body owner");
    assert_ne!(heading_owner, panel_owner);
    assert_ne!(body_owner, panel_owner);
    assert_ne!(heading_owner, body_owner);
}

#[test]
fn remove_disposes_composite_subtree() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().child(LabelledPanel {
        title: "Title",
        body: "Body",
    }));

    assert_eq!(cx.len(), 4);

    let panel = cx.get(root).expect("root").children()[0];
    assert!(cx.remove(panel));
    assert_eq!(cx.len(), 1);
    assert!(cx.get(root).expect("root").children().is_empty());
}
