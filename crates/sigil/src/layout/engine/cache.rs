use crate::geometry::Size;
use crate::kernel::{Cx, NodeId};
use crate::layout::cache_key::{
    AvailableKey, CacheDecision, LayoutCacheKey, animation_signature, layout_input_signature,
    reactive_layout_signature, seed_signature,
};

use super::LayoutEngine;

impl LayoutEngine {
    pub(super) fn prepare_animations(&mut self, cx: &Cx, root: NodeId) {
        let _prepare = crate::profile::span(crate::profile::Phase::LayoutPrepare);
        let force = self
            .cached_key
            .is_none_or(|key| key.cx_generation != cx.mutation_generation())
            || cx.dirty().any(|id| id == root);
        for key in self.reactive_layout.iter().copied() {
            let id = NodeId::from_key(key);
            let Some(signature) = cx.get(id).and_then(animation_signature) else {
                continue;
            };
            if !force && self.prepared.get(key) == Some(&signature) {
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
            let sig = seed_signature(node, self.prepared.get(key).copied());
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
