use crate::geometry::Size;
use crate::kernel::{Cx, Node, NodeId};
use crate::layout::cache_key::{
    AvailableKey, CacheDecision, LayoutCacheKey, layout_input_signature, reactive_layout_signature,
};
use ahash::AHashSet;

use super::LayoutEngine;

impl LayoutEngine {
    pub(super) fn prepare_animations(&mut self, cx: &Cx, root: NodeId) {
        let _prepare = crate::profile::span(crate::profile::Phase::LayoutPrepare);
        let force = AnimationForce::new(self.cached_key, cx, root);
        for key in self.reactive_layout.iter().copied() {
            let id = NodeId::from_key(key);
            let Some(signature) = cx.get(id).and_then(Node::animation_signature) else {
                continue;
            };
            if !force.covers(cx, id) && self.prepared.get(key) == Some(&signature) {
                continue;
            }
            crate::animation_runtime::style::prepare(cx, id);
            self.prepared.insert(key, signature);
        }
    }

    fn reactive_unchanged(&self, cx: &Cx) -> bool {
        let _sig = crate::profile::span(crate::profile::Phase::LayoutSig);
        reactive_layout_signature(cx, &self.reactive_layout) == self.last_reactive_sig
    }

    pub(super) fn resolve_cache_key(
        &self,
        cx: &Cx,
        root: NodeId,
        available: Size,
    ) -> CacheDecision {
        let cx_instance = cx.instance_id();
        let cx_generation = cx.mutation_generation();
        let available_key = AvailableKey::from_size(available);
        let scale_bits = cx.scale().factor().to_bits();
        let fonts = self.drive.font_generation();
        let key = |input_signature| LayoutCacheKey {
            cx_instance,
            root,
            available: available_key,
            scale_bits,
            fonts,
            cx_generation,
            input_signature,
        };
        let target = key(0);
        let prev = self.cached_key.filter(|prev| same_target(prev, &target));
        if let Some(prev) = prev {
            if cx.has_specific_dirty() {
                return CacheDecision::Miss(key(prev.input_signature));
            }
            if prev.cx_generation == cx_generation && self.reactive_unchanged(cx) {
                return CacheDecision::Hit;
            }
        }
        let signature = {
            let _sig = crate::profile::span(crate::profile::Phase::LayoutSig);
            layout_input_signature(cx, root)
        };
        if prev.is_some_and(|prev| prev.input_signature == signature) {
            return CacheDecision::Hit;
        }
        CacheDecision::Miss(key(signature))
    }

    pub(super) fn seed_reactive_dirty(
        &mut self,
        cx: &Cx,
        dirty: &mut Vec<NodeId>,
        mirrored: bool,
        mirror_current: bool,
    ) {
        let _seed = crate::profile::span(crate::profile::Phase::LayoutSeed);
        self.steady.clear();
        if !mirrored || mirror_current {
            return;
        }
        for key in self.reactive_layout.iter().copied() {
            let id = NodeId::from_key(key);
            let Some(node) = cx.get(id) else {
                continue;
            };
            let sig = node.seed_signature(self.prepared.get(key).copied());
            let unchanged = self.reactive_sigs.insert(key, sig) == Some(sig);
            if unchanged && self.synced_version.get(key).copied() == Some(node.version()) {
                self.steady.insert(id);
            }
            if unchanged {
                continue;
            }
            if !dirty.contains(&id) {
                dirty.push(id);
            }
        }
    }
}

enum AnimationForce {
    None,
    All,
    Under(AHashSet<NodeId>),
}

impl AnimationForce {
    fn new(cached: Option<LayoutCacheKey>, cx: &Cx, root: NodeId) -> Self {
        let Some(key) = cached else {
            return Self::All;
        };
        if cx.dirty_all() {
            return Self::All;
        }
        let dirty: AHashSet<NodeId> = cx.dirty().collect();
        if dirty.contains(&root) {
            return Self::All;
        }
        if key.cx_generation == cx.mutation_generation() {
            return Self::None;
        }
        Self::Under(dirty)
    }

    fn covers(&self, cx: &Cx, id: NodeId) -> bool {
        match self {
            Self::None => false,
            Self::All => true,
            Self::Under(dirty) => {
                std::iter::successors(Some(id), |&at| cx.parent(at)).any(|at| dirty.contains(&at))
            }
        }
    }
}

fn same_target(prev: &LayoutCacheKey, next: &LayoutCacheKey) -> bool {
    prev.cx_instance == next.cx_instance
        && prev.root == next.root
        && prev.available == next.available
        && prev.scale_bits == next.scale_bits
        && prev.fonts == next.fonts
}

pub(super) fn take_cache_miss(cx: &mut Cx, decision: CacheDecision) -> Option<LayoutCacheKey> {
    match decision {
        CacheDecision::Hit => {
            crate::profile::count(crate::profile::Counter::LayoutHit);
            cx.clear_dirty();
            None
        }
        CacheDecision::Miss(key) => {
            crate::profile::count(crate::profile::Counter::LayoutMiss);
            Some(key)
        }
    }
}
