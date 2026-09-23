//! Test helpers for driving a tree without a window.
use crate::prelude::{Cx, NodeId};
use crate::reactive::Runtime;

pub fn with_cx<F: FnOnce(&mut Cx)>(name: &'static str, f: F) {
    Runtime::scope(name, || {
        let mut cx = Cx::new_root();
        f(&mut cx);
    });
}

pub fn with_runtime<F: FnOnce(&Runtime)>(name: &'static str, f: F) {
    Runtime::scope(name, || f(&Runtime));
}

pub fn tree_size(name: &'static str, f: impl FnOnce(&mut Cx) -> NodeId) -> usize {
    let mut size = 0;
    with_cx(name, |cx| {
        let root = f(cx);
        cx.visit_preorder(root, |_, _| size += 1);
    });
    size
}

#[cfg(test)]
#[path = "testing_tests.rs"]
mod tests;
