use crate::hal::Size;
use crate::kernel::{Cx, NodeId};
use crate::style::{StyleBinding, TextStyle};
use ahash::AHashSet;
use slotmap::SecondaryMap;

use crate::layout::cache_key::{
    AvailableKey, CacheDecision, LayoutCacheKey, has_unversioned_layout, layout_input_signature,
    reactive_layout_signature, reactive_seed_signature,
};
use crate::layout::engine_drive::{LayoutDrive, NodeHandle};
use crate::layout::text_style::InheritedText;
use crate::layout::{LayoutError, LayoutResult, TextMeasure};

pub struct LayoutEngine {
    drive: LayoutDrive,
    core_to_handle: SecondaryMap<slotmap::DefaultKey, NodeHandle>,
    synced_version: SecondaryMap<slotmap::DefaultKey, u32>,
    synced_text_sig: SecondaryMap<slotmap::DefaultKey, u64>,
    published_result: SecondaryMap<slotmap::DefaultKey, LayoutResult>,
    generation: u64,
    cached_key: Option<LayoutCacheKey>,
    reactive_layout: AHashSet<slotmap::DefaultKey>,
    positions: SecondaryMap<slotmap::DefaultKey, (f32, f32)>,
    text_sizes: SecondaryMap<slotmap::DefaultKey, (u32, u32)>,
    reactive_sigs: SecondaryMap<slotmap::DefaultKey, u64>,
    steady: AHashSet<NodeId>,
    restyled: AHashSet<NodeId>,
    last_reactive_sig: u64,
}

impl LayoutEngine {
    #[must_use]
    pub fn with_text_measure(measurer: Box<dyn TextMeasure>) -> Self {
        Self {
            drive: LayoutDrive::new(measurer),
            core_to_handle: SecondaryMap::new(),
            synced_version: SecondaryMap::new(),
            synced_text_sig: SecondaryMap::new(),
            published_result: SecondaryMap::new(),
            generation: 0,
            cached_key: None,
            reactive_layout: AHashSet::new(),
            positions: SecondaryMap::new(),
            text_sizes: SecondaryMap::new(),
            reactive_sigs: SecondaryMap::new(),
            steady: AHashSet::new(),
            restyled: AHashSet::new(),
            last_reactive_sig: 0,
        }
    }

    #[must_use]
    pub fn mirrored_node_count(&self) -> usize {
        self.core_to_handle.len()
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub fn result(&self, id: NodeId) -> Option<LayoutResult> {
        self.fresh(id)
            .or_else(|| self.published_result.get(id.key()).copied())
    }

    fn fresh(&self, id: NodeId) -> Option<LayoutResult> {
        let handle = self.core_to_handle.get(id.key()).copied()?;
        let layout = self.drive.computed_box(handle)?;
        let clips = self.drive.clips_content(handle);
        Some(LayoutResult::from_resolved_box(&layout, clips))
    }

    #[must_use]
    pub fn parent_content_box(&self, cx: &Cx, id: NodeId) -> Option<Size> {
        let parent = cx.parent(id)?;
        let result = self.result(parent)?;
        let rect = result.rect();
        Some(Size::fixed(rect.width, rect.height))
    }

    pub fn sync(&mut self, cx: &Cx, root: NodeId) -> Result<(), LayoutError> {
        self.gc(cx)?;
        let root_text = TextStyle::default();
        self.sync_subtree(cx, root, InheritedText::new(&root_text))
            .map(|_handles| ())
    }

    pub fn compute(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        if cx.get(root).is_none() {
            return Err(LayoutError::NodeMissing(root));
        }
        cx.flush_reactive_children();
        let Some(cache_key) = take_cache_miss(cx, self.resolve_cache_key(cx, root, available))
        else {
            return Ok(());
        };
        if self
            .cached_key
            .is_some_and(|prev| prev.cx_instance != cache_key.cx_instance)
        {
            self.reset_mirror();
        }
        self.drive.begin_frame();
        let mut dirty = cx.dirty().collect::<Vec<_>>();
        let live_count = cx.len();
        let mirrored = self.core_to_handle.contains_key(root.key());
        let structural = self
            .cached_key
            .is_none_or(|key| key.cx_generation != cx.mutation_generation());

        let mirror_current = dirty.is_empty() && self.same_inputs(cache_key);
        self.seed_reactive_dirty(cx, &mut dirty, mirrored, mirror_current);
        if mirrored && self.translate_moved(cx, &mut dirty)? > 0 && dirty.is_empty() {
            self.finish(cx, cache_key);
            return Ok(());
        }
        let broadcast = !mirrored || dirty.is_empty() || dirty.len() >= live_count.max(1) / 2;
        if mirror_current {
            crate::profile::count("layout.resize");
            self.remeasure_and_publish(cx, root, available)?;
        } else if broadcast {
            note_full(mirrored, &dirty);
            self.full_sync_and_measure(cx, root, available)?;
        } else {
            crate::profile::count("layout.incr");
            self.incremental_sync_and_measure(cx, &dirty, root, available, structural)?;
        }

        self.finish(cx, cache_key);
        Ok(())
    }

    fn same_inputs(&self, cache_key: LayoutCacheKey) -> bool {
        self.cached_key.is_some_and(|prev| {
            prev.cx_instance == cache_key.cx_instance
                && prev.root == cache_key.root
                && prev.input_signature == cache_key.input_signature
        })
    }

    fn finish(&mut self, cx: &mut Cx, cache_key: LayoutCacheKey) {
        cx.clear_dirty();
        self.generation = self.generation.wrapping_add(1);
        self.cached_key = self
            .reactive_layout
            .iter()
            .all(|key| {
                cx.get(NodeId::from_key(*key)).is_some_and(|node| {
                    self.reactive_sigs.get(*key).copied() == Some(reactive_seed_signature(node))
                })
            })
            .then_some(cache_key);
        let _sig = crate::profile::span("layout.sig");
        self.last_reactive_sig = reactive_layout_signature(cx, &self.reactive_layout);
    }

    pub fn gc(&mut self, cx: &Cx) -> Result<(), LayoutError> {
        let stale = self
            .core_to_handle
            .keys()
            .map(NodeId::from_key)
            .filter(|id| cx.get(*id).is_none())
            .collect::<Vec<_>>();

        for id in stale {
            if self.core_to_handle.contains_key(id.key()) {
                self.remove_mirror(id)?;
            }
        }
        Ok(())
    }

    fn remove_mirror(&mut self, id: NodeId) -> Result<(), LayoutError> {
        let handle = self
            .core_to_handle
            .get(id.key())
            .copied()
            .ok_or(LayoutError::NodeMissing(id))?;
        let subtree = self.drive.subtree_handles(handle)?;

        for h in &subtree {
            if let Some(core_id) = self.drive.kernel_for(*h) {
                let _ = self.core_to_handle.remove(core_id.key());
                let _ = self.synced_version.remove(core_id.key());
                let _ = self.synced_text_sig.remove(core_id.key());
                let _ = self.published_result.remove(core_id.key());
                self.reactive_layout.remove(&core_id.key());
            }
        }
        self.drive.remove(handle)
    }

    fn resolve_cache_key(&self, cx: &Cx, root: NodeId, available: Size) -> CacheDecision {
        let cx_instance = cx.instance_id();
        let cx_generation = cx.mutation_generation();
        let available_key = AvailableKey::from_size(available);
        let scale_bits = cx.scale().factor().to_bits();
        let key = |input_signature| LayoutCacheKey {
            cx_instance,
            root,
            available: available_key,
            scale_bits,
            cx_generation,
            input_signature,
        };
        if let Some(prev) = self.cached_key
            && prev.cx_instance == cx_instance
            && prev.root == root
            && prev.available == available_key
            && prev.scale_bits == scale_bits
        {
            if cx.has_specific_dirty() {
                return CacheDecision::Miss(key(prev.input_signature));
            }
            if prev.cx_generation == cx_generation
                && reactive_layout_signature(cx, &self.reactive_layout) == self.last_reactive_sig
            {
                return CacheDecision::Hit;
            }
        }
        let signature = layout_input_signature(cx, root);
        if self.cached_key.is_some_and(|prev| {
            prev.cx_instance == cx_instance
                && prev.root == root
                && prev.available == available_key
                && prev.scale_bits == scale_bits
                && prev.input_signature == signature
        }) {
            return CacheDecision::Hit;
        }
        CacheDecision::Miss(key(signature))
    }

    fn reset_mirror(&mut self) {
        self.synced_version.clear();
        self.synced_text_sig.clear();
        self.published_result.clear();
        self.core_to_handle.clear();
        self.reactive_layout.clear();
        self.positions.clear();
        self.text_sizes.clear();
        self.reactive_sigs.clear();
        self.last_reactive_sig = 0;
    }

    fn seed_reactive_dirty(
        &mut self,
        cx: &Cx,
        dirty: &mut Vec<NodeId>,
        mirrored: bool,
        mirror_current: bool,
    ) {
        self.steady.clear();
        if !mirrored || mirror_current {
            return;
        }
        for key in self.reactive_layout.iter().copied() {
            let id = NodeId::from_key(key);
            let Some(node) = cx.get(id) else {
                continue;
            };
            let sig = reactive_seed_signature(node);
            let unchanged = self.reactive_sigs.insert(key, sig) == Some(sig);
            if unchanged {
                self.steady.insert(id);
            }
            if unchanged && !animated(node) {
                continue;
            }
            if !dirty.contains(&id) {
                dirty.push(id);
            }
        }
    }

    fn track_reactive(&mut self, id: NodeId, node: &crate::kernel::Node) {
        if has_unversioned_layout(node) {
            self.reactive_layout.insert(id.key());
            self.reactive_sigs
                .insert(id.key(), reactive_seed_signature(node));
        } else {
            self.reactive_layout.remove(&id.key());
            self.reactive_sigs.remove(id.key());
        }
        if let Some(pos) = node.get_extension::<crate::layout::ReactivePosition>() {
            self.positions.insert(id.key(), pos.get());
        }
    }

    fn handle(&self, id: NodeId) -> Result<NodeHandle, LayoutError> {
        self.core_to_handle
            .get(id.key())
            .copied()
            .ok_or(LayoutError::NodeMissing(id))
    }
}

fn take_cache_miss(cx: &mut Cx, decision: CacheDecision) -> Option<LayoutCacheKey> {
    match decision {
        CacheDecision::Hit => {
            crate::profile::count("layout.hit");
            cx.clear_dirty();
            None
        }
        CacheDecision::Miss(key) => {
            crate::profile::count("layout.miss");
            Some(key)
        }
    }
}

mod incremental;
mod measure;
mod overlays;
mod sync;

fn animated(node: &crate::kernel::Node) -> bool {
    node.get_extension::<StyleBinding>().is_some_and(|binding| {
        binding.animator_overlay().is_some() || binding.state_overlay().is_some()
    })
}

fn note_full(mirrored: bool, dirty: &[NodeId]) {
    crate::profile::count("layout.full");
    crate::profile::count(if !mirrored {
        "layout.full.unmirrored"
    } else if dirty.is_empty() {
        "layout.full.nodirty"
    } else {
        "layout.full.majority"
    });
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
