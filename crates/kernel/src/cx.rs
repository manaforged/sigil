//! Composition context and arena storage.

use core::fmt::Write as _;
use core::sync::atomic::{AtomicU64, Ordering};

use ahash::{AHashMap, AHashSet};
use sigil_hal::Scale;
use sigil_reactive::{Owner, OwnerId};
use slotmap::{DefaultKey, SlotMap};
use smallvec::SmallVec;

use crate::component::{ErasedComponent, IntoNode};
use crate::{A11yProps, Node, NodeId, NodeType, TextContent};

/// Node arena and composition context.
///
/// A `Cx` owns the UI tree and a root reactive owner. Every call to
/// [`Cx::add`] creates a per-component owner below the current owner,
/// runs the component build inside it, and records that owner for
/// subtree disposal.
pub struct Cx {
    instance_id: u64,
    nodes: SlotMap<DefaultKey, Node>,
    owner_for_node: AHashMap<NodeId, Owner>,
    dirty: AHashSet<NodeId>,
    /// When set, every live node is considered dirty without populating
    /// `dirty` with 100k entries. `mark_mutated` flips this on in O(1)
    /// instead of doing an O(N) `AHashSet::insert` loop. The dirty
    /// iterator yields all slotmap keys when this is true, then falls
    /// back to the regular set otherwise. ε1.5.
    dirty_all: bool,
    root_owner: Owner,
    roots: SmallVec<[NodeId; 4]>,
    mutation_generation: u64,
    scale: Scale,
}

impl Cx {
    /// Create an empty root context.
    #[must_use]
    pub fn new_root() -> Self {
        let root_owner = Owner::new("sigil-kernel::root", Owner::current());
        Self {
            instance_id: next_instance_id(),
            nodes: SlotMap::with_key(),
            owner_for_node: AHashMap::new(),
            dirty: AHashSet::new(),
            dirty_all: false,
            root_owner,
            roots: SmallVec::new(),
            mutation_generation: 0,
            scale: Scale::ONE,
        }
    }

    /// Return the reactive owner for this context's root.
    #[must_use]
    pub const fn root_owner(&self) -> OwnerId {
        self.root_owner.id()
    }

    /// Stable identity for this `Cx` instance.
    ///
    /// Cache owners use this to distinguish two fresh contexts whose
    /// slotmap node keys and mutation generations happen to match.
    #[must_use]
    pub const fn instance_id(&self) -> u64 {
        self.instance_id
    }

    /// Add a component as a root-level node and return its root ID.
    #[must_use]
    pub fn add<C: IntoNode>(&mut self, child: C) -> NodeId {
        self.add_with_parent(None, |cx| child.build(cx))
    }

    /// Add a component as a child of `parent` and return its root ID.
    ///
    /// Use this inside a [`IntoNode::build`] impl that manually
    /// composes subtrees. The child runs inside a fresh per-component
    /// owner parented to the current owner, and its root node is
    /// linked as a child of `parent` in the node arena.
    #[must_use]
    pub fn add_child<C: IntoNode>(&mut self, parent: NodeId, child: C) -> NodeId {
        self.add_with_parent(Some(parent), |cx| child.build(cx))
    }

    /// Create a single node in the arena and return its ID.
    ///
    /// Call this inside an [`IntoNode::build`] implementation to allocate
    /// the node for your component. Must be called within the owner scope
    /// that [`Cx::add`] or [`Cx::add_child`] sets up — i.e. inside `build`.
    ///
    /// # Panics
    ///
    /// Panics when called outside any reactive owner scope. This cannot
    /// happen if you call it from within `IntoNode::build`.
    pub fn new_node(&mut self, kind: NodeType, a11y: A11yProps) -> NodeId {
        let owner = Owner::current().expect("Cx::new_node called outside an IntoNode::build scope");
        let id = NodeId(self.nodes.insert(Node::new(kind, a11y, owner)));
        self.mark_dirty(id);
        self.bump_generation();
        id
    }

    pub(crate) fn add_boxed_child(
        &mut self,
        parent: NodeId,
        child: Box<dyn ErasedComponent>,
    ) -> NodeId {
        self.add_with_parent(Some(parent), |cx| child.build_box(cx))
    }

    fn add_with_parent(
        &mut self,
        parent_node: Option<NodeId>,
        build: impl FnOnce(&mut Self) -> NodeId,
    ) -> NodeId {
        let parent_owner = Owner::current().unwrap_or_else(|| self.root_owner.id());
        let owner = Owner::new("sigil-kernel::component", Some(parent_owner));
        let node_id = owner.with(|| build(self));
        self.owner_for_node.insert(node_id, owner);
        self.mark_dirty(node_id);
        if let Some(parent) = parent_node {
            self.link_parent(parent, node_id);
        } else if !self.roots.contains(&node_id) {
            self.roots.push(node_id);
            self.mark_dirty(node_id);
            self.bump_generation();
        }
        node_id
    }

    fn link_parent(&mut self, parent: NodeId, child: NodeId) {
        if let Some(child_node) = self.nodes.get_mut(child.key()) {
            child_node.parent = Some(parent);
            child_node.bump_version();
        }
        if let Some(parent_node) = self.nodes.get_mut(parent.key()) {
            parent_node.children.push(child);
            parent_node.bump_version();
        }
        self.roots.retain(|root| *root != child);
        self.mark_dirty(child);
        self.mark_dirty(parent);
        self.bump_generation();
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

        self.bump_generation();
        let parent = self.nodes.get(id.key()).and_then(|node| node.parent);
        let subtree = self.collect_subtree(id);
        self.detach_from_parent(id);
        if let Some(parent) = parent {
            self.mark_dirty(parent);
        }
        for node_id in &subtree {
            self.dirty.remove(node_id);
        }

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
            parent_node.bump_version();
        } else {
            self.roots.retain(|root| *root != id);
        }
    }

    /// Borrow a node by ID.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.key())
    }

    /// Mutably borrow a node by ID.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        if self.nodes.contains_key(id.key()) {
            self.mark_dirty(id);
        }
        self.nodes.get_mut(id.key())
    }

    /// Iterate kernel `NodeId`s mutated since the last `clear_dirty()`
    /// call. When `dirty_all` is set (after `mark_mutated`), yields
    /// every live node id without ever populating the underlying
    /// `AHashSet` — O(N) iteration without the O(N) set rebuild.
    pub fn dirty(&self) -> impl Iterator<Item = NodeId> + '_ {
        let from_set = self.dirty.iter().copied();
        let from_keys: Box<dyn Iterator<Item = NodeId> + '_> = if self.dirty_all {
            Box::new(self.nodes.keys().map(NodeId))
        } else {
            Box::new(std::iter::empty())
        };
        from_keys.chain(from_set)
    }

    /// Reset the dirty set. Called by layout engine + render pipeline
    /// after they sync against the set.
    pub fn clear_dirty(&mut self) {
        self.dirty.clear();
        self.dirty_all = false;
    }

    /// Explicitly invalidate caches keyed on tree or node metadata.
    ///
    /// `get_mut` is a plain borrow so projection writes, such as
    /// layout-result attachment, do not churn mutation generation. Call
    /// this after direct field-level edits that affect routing, focus, or
    /// other generation-keyed caches. ε1.5: O(1) — flips `dirty_all`
    /// instead of inserting every node id into the set (was 5-10 ms
    /// per call on 101k-node corpora).
    pub const fn mark_mutated(&mut self) {
        self.bump_generation();
        self.dirty_all = true;
    }

    /// The current per-window scale factor. Defaults to [`Scale::ONE`]
    /// until the shell adapter delivers the first
    /// [`WindowEvent::ScaleFactorChanged`](sigil_hal::WindowEvent::ScaleFactorChanged).
    #[must_use]
    pub const fn scale(&self) -> Scale {
        self.scale
    }

    /// Update the stored scale. Bumps `mutation_generation` when the factor
    /// actually changes. Re-applying the same scale is a no-op so repeated
    /// shell events do not churn caches.
    pub fn set_scale(&mut self, scale: Scale) {
        if self.scale.factor().to_bits() == scale.factor().to_bits() {
            return;
        }
        self.scale = scale;
        self.bump_generation();
        let roots = self.roots.clone();
        for root in roots {
            self.mark_dirty(root);
        }
    }

    /// Return the component owner recorded for a node root.
    #[must_use]
    pub fn component_owner(&self, id: NodeId) -> Option<OwnerId> {
        self.owner_for_node.get(&id).map(|owner| owner.id())
    }

    /// Return the root nodes in insertion order.
    #[must_use]
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    /// Return the monotonic tree mutation generation.
    #[must_use]
    pub const fn mutation_generation(&self) -> u64 {
        self.mutation_generation
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

    /// Return the parent of a node.
    #[must_use]
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id.key()).and_then(|node| node.parent)
    }

    /// Return the first child of a node.
    #[must_use]
    pub fn first_child(&self, id: NodeId) -> Option<NodeId> {
        self.nodes
            .get(id.key())
            .and_then(|node| node.children.first().copied())
    }

    /// Return the last child of a node.
    #[must_use]
    pub fn last_child(&self, id: NodeId) -> Option<NodeId> {
        self.nodes
            .get(id.key())
            .and_then(|node| node.children.last().copied())
    }

    /// Return the nth child (0-indexed) of a node.
    #[must_use]
    pub fn nth_child(&self, id: NodeId, n: usize) -> Option<NodeId> {
        self.nodes
            .get(id.key())
            .and_then(|node| node.children.get(n).copied())
    }

    /// Return the number of children of a node.
    #[must_use]
    pub fn child_count(&self, id: NodeId) -> usize {
        self.nodes
            .get(id.key())
            .map_or(0, |node| node.children.len())
    }

    fn write_dump(&self, id: NodeId, depth: usize, out: &mut String) {
        let Some(node) = self.nodes.get(id.key()) else {
            return;
        };
        for _ in 0..depth {
            out.push_str("  ");
        }
        match &node.kind {
            NodeType::Container => {
                let _ = writeln!(out, "Container role={:?}", node.a11y.role);
            }
            NodeType::Fragment => {
                let _ = writeln!(out, "Fragment role={:?}", node.a11y.role);
            }
            NodeType::Text { content } => {
                let _ = writeln!(
                    out,
                    "Text {} role={:?}",
                    dump_text_content(content),
                    node.a11y.role
                );
            }
        }
        for child in &node.children {
            self.write_dump(*child, depth + 1, out);
        }
    }

    const fn bump_generation(&mut self) {
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
    }

    fn mark_dirty(&mut self, id: NodeId) {
        self.dirty.insert(id);
    }
}

impl Drop for Cx {
    fn drop(&mut self) {
        self.root_owner.dispose();
    }
}

fn dump_text_content(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => format!("{value:?}"),
        TextContent::Reactive(_) => "<reactive>".to_owned(),
    }
}

fn next_instance_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
