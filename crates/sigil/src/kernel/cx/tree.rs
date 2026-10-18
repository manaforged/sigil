use crate::kernel::{Cx, Node, NodeId};

impl Cx {
    pub fn visit_preorder(&self, root: NodeId, mut visit: impl FnMut(NodeId, &Node)) {
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get(id.key()) else {
                continue;
            };
            visit(id, node);
            stack.extend(node.children.iter().rev().copied());
        }
    }

    #[must_use]
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id.key()).and_then(|node| node.parent)
    }

    #[must_use]
    pub fn first_child(&self, id: NodeId) -> Option<NodeId> {
        self.nodes
            .get(id.key())
            .and_then(|node| node.children.first().copied())
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn nth_child(&self, id: NodeId, n: usize) -> Option<NodeId> {
        self.nodes
            .get(id.key())
            .and_then(|node| node.children.get(n).copied())
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn child_count(&self, id: NodeId) -> usize {
        self.nodes
            .get(id.key())
            .map_or(0, |node| node.children.len())
    }
}
