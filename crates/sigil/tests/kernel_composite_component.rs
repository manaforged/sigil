#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use sigil::kernel::{Cx, IntoNode, NodeId, Role, container, text};

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
fn add_child_links_parent() {
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
