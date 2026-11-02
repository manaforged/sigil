use crate::kernel::{Cx, container};
use crate::reactive::Owner;

#[test]
fn describe_names_a_node_by_role_and_label() {
    Owner::scope("describe", || {
        let mut cx = Cx::new_root();
        let root = cx.add(container().child(crate::widgets::button("Start")));
        let mut names = Vec::new();
        cx.visit_preorder(root, |id, _| names.push(id));
        let described: Vec<String> = names.into_iter().map(|id| cx.describe(id)).collect();
        assert!(
            described.iter().any(|name| name == "Button \"Start\""),
            "{described:?}"
        );
    });
}
