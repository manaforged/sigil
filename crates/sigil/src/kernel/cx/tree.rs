use crate::kernel::{Cx, Node, NodeId};

impl Cx {
    /// Calls `visit` on `root` and each descendant, parents first.
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

    /// Returns the parent of this node.
    #[must_use]
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id.key()).and_then(|node| node.parent)
    }

    /// Returns the first child of this node.
    #[must_use]
    #[doc(hidden)]
    #[cfg(feature = "kittest")]
    pub fn first_child(&self, id: NodeId) -> Option<NodeId> {
        self.nodes
            .get(id.key())
            .and_then(|node| node.children.first().copied())
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn nth_child(&self, id: NodeId, n: usize) -> Option<NodeId> {
        self.nodes
            .get(id.key())
            .and_then(|node| node.children.get(n).copied())
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn child_count(&self, id: NodeId) -> usize {
        self.nodes
            .get(id.key())
            .map_or(0, |node| node.children.len())
    }
}

impl core::fmt::Debug for Cx {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Cx")
            .field("instance_id", &self.instance_id)
            .field("roots", &self.roots)
            .field("scale", &self.scale)
            .field("host_mode", &self.host_mode)
            .field("live_region", &self.live_region)
            .finish_non_exhaustive()
    }
}
