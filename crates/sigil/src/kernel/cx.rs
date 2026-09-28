use core::sync::atomic::{AtomicU64, Ordering};

use crate::hal::Scale;
use crate::reactive::{Owner, OwnerId};
use ahash::{AHashMap, AHashSet};
use slotmap::{DefaultKey, SlotMap};
use smallvec::SmallVec;

use crate::kernel::component::{ErasedComponent, IntoNode};
use crate::kernel::{A11yProps, Node, NodeId, NodeType};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HostMode {
    #[default]
    Pixel,
    Cell,
}

pub struct Cx {
    instance_id: u64,
    pub(crate) nodes: SlotMap<DefaultKey, Node>,
    pub(crate) owner_for_node: AHashMap<NodeId, Owner>,
    pub(crate) dirty: AHashSet<NodeId>,
    dirty_all: bool,
    root_owner: Owner,
    pub(crate) roots: SmallVec<[NodeId; 4]>,
    pub(crate) mutation_generation: u64,
    scale: Scale,
    host_mode: HostMode,
    pub(crate) live_region: Option<NodeId>,
}

impl Cx {
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
            host_mode: HostMode::default(),
            live_region: None,
        }
    }

    #[must_use]
    pub const fn host_mode(&self) -> HostMode {
        self.host_mode
    }

    pub(crate) const fn set_host_mode(&mut self, mode: HostMode) {
        self.host_mode = mode;
    }

    #[must_use]
    pub const fn root_owner(&self) -> OwnerId {
        self.root_owner.id()
    }

    #[must_use]
    pub const fn instance_id(&self) -> u64 {
        self.instance_id
    }

    #[must_use]
    pub fn add<C: IntoNode>(&mut self, child: C) -> NodeId {
        self.add_with_parent(None, |cx| child.build(cx))
    }

    #[must_use]
    pub fn add_child<C: IntoNode>(&mut self, parent: NodeId, child: C) -> NodeId {
        self.add_with_parent(Some(parent), |cx| child.build(cx))
    }

    pub fn new_node(&mut self, kind: NodeType, a11y: A11yProps) -> NodeId {
        let owner = Owner::current().unwrap_or_else(|| {
            tracing::warn!(
                target: "sigil::kernel",
                "Cx::new_node called outside an IntoNode::build scope; using the root owner"
            );
            self.root_owner()
        });
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

    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.key())
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        if self.nodes.contains_key(id.key()) {
            self.mark_dirty(id);
        }
        self.nodes.get_mut(id.key())
    }

    pub fn get_mut_untracked(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.key())
    }

    pub fn dirty(&self) -> impl Iterator<Item = NodeId> + '_ {
        let from_set = self.dirty.iter().copied();
        let from_keys: Box<dyn Iterator<Item = NodeId> + '_> = if self.dirty_all {
            Box::new(self.nodes.keys().map(NodeId))
        } else {
            Box::new(std::iter::empty())
        };
        from_keys.chain(from_set)
    }

    #[must_use]
    pub const fn dirty_all(&self) -> bool {
        self.dirty_all
    }

    #[must_use]
    pub fn has_specific_dirty(&self) -> bool {
        !self.dirty_all && !self.dirty.is_empty()
    }

    pub fn clear_dirty(&mut self) {
        self.dirty.clear();
        self.dirty_all = false;
    }

    pub const fn mark_mutated(&mut self) {
        self.bump_generation();
        self.dirty_all = true;
    }

    #[must_use]
    pub const fn scale(&self) -> Scale {
        self.scale
    }

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

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn component_owner(&self, id: NodeId) -> Option<OwnerId> {
        self.owner_for_node.get(&id).map(|owner| owner.id())
    }

    #[must_use]
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    #[must_use]
    pub const fn mutation_generation(&self) -> u64 {
        self.mutation_generation
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

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

fn next_instance_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[path = "cx_dump.rs"]
mod dump;

#[cfg(test)]
#[path = "cx_tests.rs"]
mod tests;
