#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use crate::kernel::{Cx, container as kernel_container};
use crate::reactive::Runtime;

#[test]
fn status_bar_has_no_children_when_iterator_is_empty() {
    Runtime::scope("status-bar-empty", || {
        let mut cx = Cx::new_root();
        let children: Vec<NodeBuilder> = Vec::new();
        let id = status_bar(children).build(&mut cx);
        let node = cx.get(id).expect("status-bar root exists");
        assert_eq!(node.a11y.role, Role::Status);
        assert!(
            node.children().is_empty(),
            "empty iterator must produce zero children"
        );
    });
}

#[test]
fn status_bar_forwards_each_child_in_order() {
    Runtime::scope("status-bar-children", || {
        let mut cx = Cx::new_root();
        let children = vec![kernel_container(), kernel_container()];
        let id = status_bar(children).build(&mut cx);
        let node = cx.get(id).expect("status-bar root exists");
        assert_eq!(
            node.children().len(),
            2,
            "status_bar forwards every child in the input iterator"
        );
    });
}

#[test]
fn kv_row_holds_label_and_value_children_in_order() {
    Runtime::scope("kv-row-static-label", || {
        let mut cx = Cx::new_root();
        let id = kv_row("lap 1", text("00:42.183")).build(&mut cx);
        let node = cx.get(id).expect("kv-row root exists");
        assert_eq!(node.a11y.role, Role::GenericContainer);
        assert_eq!(
            node.children().len(),
            2,
            "kv_row produces exactly two children: label then value"
        );
    });
}

#[test]
fn kv_row_accepts_owned_label_string() {
    Runtime::scope("kv-row-owned-label", || {
        let mut cx = Cx::new_root();
        let id = kv_row(String::from("status"), text("ok")).build(&mut cx);
        let node = cx.get(id).expect("kv-row root exists");
        assert_eq!(node.children().len(), 2);
    });
}
