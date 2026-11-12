use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use ahash::{AHashMap, AHashSet};

use crate::kernel::{Cx, NodeId};

static DELTA_EPOCH: AtomicU64 = AtomicU64::new(1);

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
    pub(crate) const fn layout_if(affects_layout: bool) -> Self {
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
    #[cfg(host)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.structure && self.paint.is_empty() && self.layout.is_empty()
    }

    pub(crate) fn record(&mut self, id: NodeId, kind: DeltaKind) {
        DELTA_EPOCH.fetch_add(1, Ordering::Relaxed);
        match kind {
            DeltaKind::Paint => self.paint.insert(id),
            DeltaKind::Layout => self.layout.insert(id),
        };
    }

    fn absorb(&mut self, other: Self) {
        self.paint.extend(other.paint);
        self.layout.extend(other.layout);
    }
}

impl NodeTag {
    pub(crate) const fn new(cx: u64, id: NodeId) -> Self {
        Self { cx, id }
    }

    #[cfg(feature = "kittest")]
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
    #[cfg(host)]
    #[must_use]
    pub(crate) fn paint_epoch(&self) -> u64 {
        DELTA_EPOCH
            .load(Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ self.mutation_generation()
    }

    pub(crate) fn take_delta(&mut self) -> FrameDelta {
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

    pub(crate) fn reuse_delta(&mut self, mut spent: FrameDelta) {
        spent.paint.clear();
        spent.layout.clear();
        let pending = std::mem::take(&mut self.delta);
        spent.structure = pending.structure;
        spent.absorb(pending);
        self.delta = spent;
    }
}

#[cfg(test)]
#[path = "delta_tests.rs"]
mod tests;
