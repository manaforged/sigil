#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use sigil_kernel::{Cx, NodeType, Role, container, text};

#[allow(clippy::cognitive_complexity)]
#[test]
fn builds_expected_tree_shape() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .role(Role::Main)
            .child(text("Hello"))
            .child(container().child(text("Nested")))
            .child(text("Tail")),
    );

    assert_eq!(cx.len(), 5);

    let root_node = cx.get(root).expect("root node exists");
    assert!(matches!(root_node.kind, NodeType::Container));
    assert_eq!(root_node.parent(), None);
    assert_eq!(root_node.children().len(), 3);

    let children = root_node.children().to_vec();
    let first = children[0];
    let nested = children[1];
    let tail = children[2];

    assert_eq!(cx.get(first).expect("first child").parent(), Some(root));
    assert_eq!(cx.get(nested).expect("nested child").parent(), Some(root));
    assert_eq!(cx.get(tail).expect("tail child").parent(), Some(root));

    match &cx.get(first).expect("first child").kind {
        NodeType::Text { content, .. } => assert_eq!(content.as_static_str(), Some("Hello")),
        other => panic!("expected text child, got {other:?}"),
    }

    let nested_node = cx.get(nested).expect("nested container");
    assert!(matches!(nested_node.kind, NodeType::Container));
    assert_eq!(nested_node.children().len(), 1);

    let nested_text = nested_node.children()[0];
    assert_eq!(
        cx.get(nested_text).expect("nested text").parent(),
        Some(nested)
    );
    match &cx.get(nested_text).expect("nested text").kind {
        NodeType::Text { content, .. } => assert_eq!(content.as_static_str(), Some("Nested")),
        other => panic!("expected nested text, got {other:?}"),
    }

    let expected = concat!(
        "Container role=Main\n",
        "  Text \"Hello\" role=Label\n",
        "  Container role=GenericContainer\n",
        "    Text \"Nested\" role=Label\n",
        "  Text \"Tail\" role=Label\n",
    );
    assert_eq!(cx.dump_tree(root), expected);
}
