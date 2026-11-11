use core::sync::atomic::{AtomicU64, Ordering};

use crate::geometry::Scale;
use crate::reactive::Owner;
use ahash::{AHashMap, AHashSet};
use slotmap::{DefaultKey, SlotMap};
use smallvec::SmallVec;

use crate::kernel::component::{ErasedComponent, IntoNode};
use crate::kernel::delta::{DeltaKind, FrameDelta, NodeTag};
use crate::kernel::{A11yProps, Node, NodeId, NodeType};

/// Tells widgets whether the host lays out in pixels (window) or in terminal cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostMode {
    /// The pixel host mode.
    #[default]
    Pixel,
    /// The cell host mode.
    Cell,
}

/// The node tree.
pub struct Cx {
    instance_id: u64,
    pub(crate) nodes: SlotMap<DefaultKey, Node>,
    pub(crate) owner_for_node: AHashMap<NodeId, Owner>,
    pub(crate) dirty: AHashSet<NodeId>,
    dirty_all: bool,
    pub(crate) delta: FrameDelta,
    pub(crate) delta_generation: u64,
    root_owner: Owner,
    pub(crate) roots: SmallVec<[NodeId; 4]>,
    pub(crate) mutation_generation: u64,
    scale: f32,
    host_mode: HostMode,
    pub(crate) live_region: Option<NodeId>,
    pub(crate) top_layer_memo: super::top_layer::TopLayerMemo,
    marked: std::cell::RefCell<marked::Marked>,
}

impl Cx {
    /// Returns the new root cx.
    #[must_use]
    pub fn new_root() -> Self {
        let root_owner = Owner::new("sigil-kernel::root", Owner::current());
        Self {
            instance_id: next_instance_id(),
            nodes: SlotMap::with_key(),
            owner_for_node: AHashMap::new(),
            dirty: AHashSet::new(),
            dirty_all: false,
            delta: FrameDelta::default(),
            delta_generation: 0,
            root_owner,
            roots: SmallVec::new(),
            mutation_generation: 0,
            scale: 1.0,
            host_mode: HostMode::default(),
            top_layer_memo: std::cell::RefCell::new(None),
            live_region: None,
            marked: std::cell::RefCell::default(),
        }
    }

    /// Returns the host mode.
    #[must_use]
    pub(crate) const fn host_mode(&self) -> HostMode {
        self.host_mode
    }

    pub(crate) const fn set_host_mode(&mut self, mode: HostMode) {
        self.host_mode = mode;
    }

    /// Returns the root owner.
    #[must_use]
    pub(crate) const fn root_owner(&self) -> Owner {
        self.root_owner
    }

    /// Returns the instance id.
    #[must_use]
    pub(crate) const fn instance_id(&self) -> u64 {
        self.instance_id
    }

    /// Builds `child` with no parent and returns its id.
    #[must_use]
    pub fn add<C: IntoNode>(&mut self, child: C) -> NodeId {
        self.add_with_parent(None, |cx| child.build(cx))
    }

    /// Builds `child` under `parent` and returns its id.
    #[must_use]
    pub fn add_child<C: IntoNode>(&mut self, parent: NodeId, child: C) -> NodeId {
        self.add_with_parent(Some(parent), |cx| child.build(cx))
    }

    kittest_pub! {
        /// Creates an empty node and returns its id.
        #[doc(hidden)]
        pub fn new_node(&mut self, kind: NodeType, a11y: A11yProps) -> NodeId {
        let owner = Owner::current().unwrap_or_else(|| {
            tracing::warn!(
                target: "sigil::kernel",
                "Cx::new_node called outside an IntoNode::build scope; using the root owner"
            );
            self.root_owner()
        });
        let cx = self.instance_id;
        let id = NodeId(self.nodes.insert_with_key(|key| {
            Node::new(kind, a11y, owner).tagged(NodeTag::new(cx, NodeId(key)))
        }));
        self.mark_dirty(id);
        self.bump_generation();
        id
        }
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
        let parent_owner = Owner::current().unwrap_or(self.root_owner);
        let owner = Owner::new("sigil-kernel::component", Some(parent_owner));
        let node_id = owner.with(|| build(self));
        self.owner_for_node.insert(node_id, owner);
        self.mark_dirty(node_id);
        if let Some(parent) = parent_node {
            self.link_parent(parent, node_id);
        } else if !self.roots.contains(&node_id) {
            self.roots.push(node_id);
            self.bump_generation();
        }
        node_id
    }

    fn link_parent(&mut self, parent: NodeId, child: NodeId) {
        if self.nodes.get(parent.key()).is_none() || self.nodes.get(child.key()).is_none() {
            tracing::warn!(
                ?parent,
                ?child,
                "link_parent skipped a dead node; the child stays unattached"
            );
            return;
        }
        self.detach_from_parent(child);
        if let Some(child_node) = self.nodes.get_mut(child.key()) {
            child_node.parent = Some(parent);
            child_node.bump_version();
        }
        if let Some(parent_node) = self.nodes.get_mut(parent.key()) {
            parent_node.children.push(child);
            parent_node.bump_version();
        }
        self.mark_dirty(child);
        self.mark_dirty(parent);
        self.bump_generation();
    }

    /// Removes a node and its subtree; returns whether it existed.
    pub fn remove(&mut self, id: NodeId) -> bool {
        if self.nodes.get(id.key()).is_none() {
            return false;
        }

        self.bump_generation();
        let subtree = self.collect_subtree(id);
        self.detach_from_parent(id);
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

    pub(crate) fn replace_children(&mut self, parent: NodeId, children: SmallVec<[NodeId; 4]>) {
        let children: SmallVec<[NodeId; 4]> = children
            .into_iter()
            .filter(|id| self.nodes.get(id.key()).is_some())
            .collect();
        let Some(parent_node) = self.nodes.get_mut(parent.key()) else {
            return;
        };
        if parent_node.children.as_slice() == children.as_slice() {
            return;
        }
        let old = std::mem::take(&mut parent_node.children);
        for id in &children {
            let prior = self.nodes.get(id.key()).and_then(|node| node.parent);
            if prior == Some(parent) {
                continue;
            }
            self.detach_from_parent(*id);
            if let Some(child) = self.nodes.get_mut(id.key()) {
                child.parent = Some(parent);
                child.bump_version();
            }
        }
        for id in &old {
            if children.contains(id) {
                continue;
            }
            if let Some(dropped) = self.nodes.get_mut(id.key())
                && dropped.parent == Some(parent)
            {
                dropped.parent = None;
                dropped.bump_version();
            }
        }
        if let Some(target) = self.nodes.get_mut(parent.key()) {
            target.children = children;
            target.bump_version();
        }
        self.mark_dirty(parent);
        self.bump_generation();
    }

    fn detach_from_parent(&mut self, id: NodeId) {
        let parent = self.nodes.get(id.key()).and_then(|node| node.parent);
        if let Some(parent) = parent
            && let Some(parent_node) = self.nodes.get_mut(parent.key())
        {
            parent_node.children.retain(|child| *child != id);
            parent_node.bump_version();
            self.mark_dirty(parent);
        } else {
            self.roots.retain(|root| *root != id);
        }
    }

    /// Returns the node with this id.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.key())
    }

    /// Returns the node with this id for writing and marks it changed.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        if self.nodes.contains_key(id.key()) {
            self.mark_dirty(id);
        }
        self.nodes.get_mut(id.key())
    }

    /// Returns the node with this id for writing without marking it changed.
    pub(crate) fn get_mut_untracked(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.key())
    }

    kittest_pub! {
        /// Returns the dirty.
        pub fn dirty(&self) -> impl Iterator<Item = NodeId> + '_ {
            self.dirty_all
                .then(|| self.nodes.keys().map(NodeId))
                .into_iter()
                .flatten()
                .chain(self.dirty.iter().copied())
        }
    }

    /// Returns whether dirty all is set.
    #[must_use]
    pub(crate) const fn dirty_all(&self) -> bool {
        self.dirty_all
    }

    /// Returns whether this cx has specific dirty.
    #[must_use]
    pub(crate) fn has_specific_dirty(&self) -> bool {
        !self.dirty_all && !self.dirty.is_empty()
    }

    kittest_pub! {
        /// Clears the set of changed nodes.
        pub fn clear_dirty(&mut self) {
            self.dirty.clear();
            self.dirty_all = false;
        }
    }

    /// Marks the tree as changed.
    #[doc(hidden)]
    #[cfg(feature = "kittest")]
    pub fn mark_mutated(&mut self) {
        self.bump_generation();
        self.dirty_all = true;
        self.marked.get_mut().rescan();
    }

    kittest_pub! {
        /// Returns the scale.
        #[must_use]
        pub const fn scale(&self) -> f32 {
            self.scale
        }
    }

    kittest_pub! {
        if feature = "desktop";
        /// Sets the scale; a non-finite or non-positive value becomes 1.0.
        pub fn set_scale(&mut self, scale: f32) {
            let scale = Scale::new(scale).factor();
            if self.scale.to_bits() == scale.to_bits() {
                return;
            }
            self.scale = scale;
            self.bump_generation();
            let roots = self.roots.clone();
            for root in roots {
                self.mark_dirty(root);
            }
        }
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn component_owner(&self, id: NodeId) -> Option<Owner> {
        self.owner_for_node.get(&id).copied()
    }

    /// Returns the roots.
    #[must_use]
    pub(crate) fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    kittest_pub! {
        /// Returns the mutation generation.
        #[must_use]
        pub const fn mutation_generation(&self) -> u64 {
            self.mutation_generation
        }
    }

    /// Returns the len.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether this cx is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    const fn bump_generation(&mut self) {
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
    }

    fn mark_dirty(&mut self, id: NodeId) {
        self.dirty.insert(id);
        self.marked.get_mut().touch(id);
        self.delta.record(id, DeltaKind::Layout);
    }
}

impl Drop for Cx {
    fn drop(&mut self) {
        self.root_owner.dispose();
        super::delta::discard_pending(self.instance_id);
    }
}

fn next_instance_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[path = "cx_dump.rs"]
mod dump;
mod marked;
mod tree;

#[cfg(test)]
#[path = "cx_tests.rs"]
mod tests;
