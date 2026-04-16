//! Composition context and arena storage.

use core::fmt::Write as _;

use ahash::AHashMap;
use sigil_reactive::{Owner, OwnerId};
use slotmap::{DefaultKey, SlotMap};

use crate::component::{BoxedComponent, Component};
use crate::{Node, NodeId, NodeKind, Semantics, TextSource};

/// Node arena and composition context.
///
/// A `Cx` owns the UI tree and a root reactive owner. Every call to
/// [`Cx::add`] creates a per-component owner below the current owner,
/// runs the component build inside it, and records that owner for
/// subtree disposal.
pub struct Cx {
    nodes: SlotMap<DefaultKey, Node>,
    owner_for_node: AHashMap<NodeId, Owner>,
    root_owner: Owner,
}

impl Cx {
    /// Create an empty root context.
    #[must_use]
    pub fn new_root() -> Self {
        let root_owner = Owner::new("sigil-core::root", Owner::current());
        Self {
            nodes: SlotMap::with_key(),
            owner_for_node: AHashMap::new(),
            root_owner,
        }
    }

    /// Return the reactive owner for this context's root.
    #[must_use]
    pub const fn root_owner(&self) -> OwnerId {
        self.root_owner.id()
    }

    /// Add a component as a root-level node and return its root ID.
    #[must_use]
    pub fn add<C: Component>(&mut self, child: C) -> NodeId {
        self.add_with_parent(None, |cx| child.build(cx))
    }

    /// Add a component as a child of `parent` and return its root ID.
    ///
    /// Use this inside a [`Component::build`] impl that manually
    /// composes subtrees. The child runs inside a fresh per-component
    /// owner parented to the current owner, and its root node is
    /// linked as a child of `parent` in the node arena.
    #[must_use]
    pub fn add_child<C: Component>(&mut self, parent: NodeId, child: C) -> NodeId {
        self.add_with_parent(Some(parent), |cx| child.build(cx))
    }

    /// Insert one raw node into the arena under the current owner.
    ///
    /// Primitive builders use this inside the owner scope established
    /// by [`Cx::add`] or [`Cx::add_child`].
    ///
    /// # Panics
    ///
    /// Panics when called outside any reactive owner scope. Raw nodes
    /// must belong to a component owner so signals, effects, and
    /// cleanups registered during build tear down with the component
    /// instead of leaking to the context root.
    pub fn insert_raw(&mut self, kind: NodeKind, semantics: Semantics) -> NodeId {
        let owner =
            Owner::current().expect("Cx::insert_raw called outside any reactive owner scope");
        NodeId(self.nodes.insert(Node::new(kind, semantics, owner)))
    }

    pub(crate) fn add_boxed_child(
        &mut self,
        parent: NodeId,
        child: Box<dyn BoxedComponent>,
    ) -> NodeId {
        self.add_with_parent(Some(parent), |cx| child.build_box(cx))
    }

    fn add_with_parent(
        &mut self,
        parent_node: Option<NodeId>,
        build: impl FnOnce(&mut Self) -> NodeId,
    ) -> NodeId {
        let parent_owner = Owner::current().unwrap_or_else(|| self.root_owner.id());
        let owner = Owner::new("sigil-core::component", Some(parent_owner));
        let node_id = owner.with(|| build(self));
        self.owner_for_node.insert(node_id, owner);
        if let Some(parent) = parent_node {
            self.link_parent(parent, node_id);
        }
        node_id
    }

    fn link_parent(&mut self, parent: NodeId, child: NodeId) {
        if let Some(child_node) = self.nodes.get_mut(child.key()) {
            child_node.parent = Some(parent);
        }
        if let Some(parent_node) = self.nodes.get_mut(parent.key()) {
            parent_node.children.push(child);
        }
    }

    /// Remove a node and all descendants from the arena.
    ///
    /// Returns `false` when `id` is stale or missing. Reactive owners
    /// are disposed in post-order, so child cleanups run before parent
    /// cleanups.
    pub fn remove(&mut self, id: NodeId) -> bool {
        if self.nodes.get(id.key()).is_none() {
            return false;
        }

        let subtree = self.collect_subtree(id);
        self.detach_from_parent(id);

        for node_id in subtree.iter().rev() {
            if let Some(owner) = self.owner_for_node.remove(node_id) {
                owner.dispose();
            }
        }
        for node_id in subtree.iter().rev() {
            let _ = self.nodes.remove(node_id.key());
        }
        true
    }

    fn collect_subtree(&self, root: NodeId) -> Vec<NodeId> {
        let mut stack = vec![root];
        let mut out = Vec::new();
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get(id.key()) else {
                continue;
            };
            out.push(id);
            stack.extend(node.children.iter().copied());
        }
        out
    }

    fn detach_from_parent(&mut self, id: NodeId) {
        let parent = self.nodes.get(id.key()).and_then(|node| node.parent);
        if let Some(parent) = parent
            && let Some(parent_node) = self.nodes.get_mut(parent.key())
        {
            parent_node.children.retain(|child| *child != id);
        }
    }

    /// Borrow a node by ID.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.key())
    }

    /// Mutably borrow a node by ID.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.key())
    }

    /// Return the component owner recorded for a node root.
    #[must_use]
    pub fn component_owner(&self, id: NodeId) -> Option<OwnerId> {
        self.owner_for_node.get(&id).map(|owner| owner.id())
    }

    /// Number of live nodes in the arena.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Return whether the arena contains no live nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Visit a subtree in preorder.
    pub fn visit_preorder(&self, root: NodeId, mut visit: impl FnMut(NodeId, &Node)) {
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get(id.key()) else {
                continue;
            };
            visit(id, node);
            for child in node.children.iter().rev().copied() {
                stack.push(child);
            }
        }
    }

    /// Dump a subtree into a stable, human-readable outline.
    #[must_use]
    pub fn dump_tree(&self, root: NodeId) -> String {
        let mut out = String::new();
        self.write_dump(root, 0, &mut out);
        out
    }

    fn write_dump(&self, id: NodeId, depth: usize, out: &mut String) {
        let Some(node) = self.nodes.get(id.key()) else {
            return;
        };
        for _ in 0..depth {
            out.push_str("  ");
        }
        match &node.kind {
            NodeKind::Container => {
                let _ = writeln!(out, "Container role={:?}", node.semantics.role);
            }
            NodeKind::Fragment => {
                let _ = writeln!(out, "Fragment role={:?}", node.semantics.role);
            }
            NodeKind::Text { source } => {
                let _ = writeln!(
                    out,
                    "Text {} role={:?}",
                    dump_text_source(source),
                    node.semantics.role
                );
            }
        }
        for child in &node.children {
            self.write_dump(*child, depth + 1, out);
        }
    }
}

impl Drop for Cx {
    fn drop(&mut self) {
        self.root_owner.dispose();
    }
}

fn dump_text_source(source: &TextSource) -> String {
    match source {
        TextSource::Static(value) => format!("{value:?}"),
        TextSource::Reactive(_) => "<reactive>".to_owned(),
    }
}
