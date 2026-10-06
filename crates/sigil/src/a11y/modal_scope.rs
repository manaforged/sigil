use std::collections::HashSet;

use accesskit::{Node as AkNode, NodeId as AkNodeId};

use crate::events::hit_test::visible;
use crate::kernel::{Cx, FocusScopeKind, NodeId, NodeType};

pub(super) fn hide_outside(cx: &Cx, nodes: &mut [(AkNodeId, AkNode)]) {
    let Some(modal) = open_modal(cx) else {
        return;
    };
    let mut hidden = HashSet::new();
    let mut child = modal;
    while let Some(parent) = cx.parent(child) {
        if let Some(node) = cx.get(parent) {
            for &sibling in node.children().iter().filter(|&&id| id != child) {
                hide(cx, sibling, &mut hidden);
            }
        }
        child = parent;
    }
    for &root in cx.roots().iter().filter(|&&id| id != child) {
        hide(cx, root, &mut hidden);
    }
    for (id, node) in nodes.iter_mut() {
        if hidden.contains(id) {
            node.set_hidden();
        }
    }
}

fn hide(cx: &Cx, id: NodeId, hidden: &mut HashSet<AkNodeId>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if matches!(node.kind, NodeType::Fragment) {
        for &child in node.children() {
            hide(cx, child, hidden);
        }
    } else {
        hidden.insert(id.to_accesskit());
    }
}

fn open_modal(cx: &Cx) -> Option<NodeId> {
    let mut found = None;
    let mut stack: Vec<NodeId> = cx.roots().iter().rev().copied().collect();
    while let Some(id) = stack.pop() {
        let Some(node) = cx.get(id) else {
            continue;
        };
        if node.a11y.focus_scope == Some(FocusScopeKind::Modal) && visible(cx, id) {
            found = Some(id);
        }
        stack.extend(node.children().iter().rev().copied());
    }
    found
}
