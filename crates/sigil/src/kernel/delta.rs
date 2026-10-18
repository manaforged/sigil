use std::cell::RefCell;

use ahash::{AHashMap, AHashSet};

use crate::kernel::{Cx, NodeId};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameDelta {
    pub paint: AHashSet<NodeId>,
    pub layout: AHashSet<NodeId>,
    pub structure: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeltaKind {
    Paint,
    Layout,
}

impl DeltaKind {
    #[must_use]
    pub const fn layout_if(affects_layout: bool) -> Self {
        if affects_layout {
            Self::Layout
        } else {
            Self::Paint
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeTag {
    cx: u64,
    id: NodeId,
}

thread_local! {
    static PENDING: RefCell<AHashMap<u64, FrameDelta>> = RefCell::new(AHashMap::new());
}

impl FrameDelta {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.structure && self.paint.is_empty() && self.layout.is_empty()
    }

    pub(crate) fn record(&mut self, id: NodeId, kind: DeltaKind) {
        match kind {
            DeltaKind::Paint => self.paint.insert(id),
            DeltaKind::Layout => self.layout.insert(id),
        };
    }

    fn absorb(&mut self, other: Self) {
        self.paint.extend(other.paint);
        self.layout.extend(other.layout);
        self.structure |= other.structure;
    }
}

impl NodeTag {
    pub(crate) const fn new(cx: u64, id: NodeId) -> Self {
        Self { cx, id }
    }

    #[must_use]
    pub const fn id(self) -> NodeId {
        self.id
    }

    pub fn record(self, kind: DeltaKind) {
        let _ = PENDING.try_with(|cell| {
            cell.borrow_mut()
                .entry(self.cx)
                .or_default()
                .record(self.id, kind);
        });
    }
}

pub(crate) fn discard_pending(cx: u64) {
    let _ = PENDING.try_with(|cell| cell.borrow_mut().remove(&cx));
}

impl Cx {
    pub fn take_delta(&mut self) -> FrameDelta {
        let mut delta = std::mem::take(&mut self.delta);
        if let Some(pending) = PENDING.with(|cell| cell.borrow_mut().remove(&self.instance_id())) {
            delta.absorb(pending);
        }
        delta.paint.retain(|id| self.nodes.contains_key(id.key()));
        delta.layout.retain(|id| self.nodes.contains_key(id.key()));
        let generation = self.mutation_generation();
        delta.structure |= self.delta_generation != generation;
        self.delta_generation = generation;
        delta
    }
}

#[cfg(test)]
#[path = "delta_tests.rs"]
mod tests;
