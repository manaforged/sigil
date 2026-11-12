use crate::geometry::Constraint;
use crate::kernel::{Cx, NodeId};
use crate::style::TextStyle;
use ahash::AHashSet;
use slotmap::SecondaryMap;
use smallvec::SmallVec;
use std::collections::BTreeSet;

use crate::Error;
use crate::layout::cache_key::{LayoutCacheKey, reactive_layout_signature};
use crate::layout::engine_drive::{LayoutDrive, NodeHandle};
use crate::layout::shift::Shift;
use crate::layout::text_style::InheritedText;
use crate::layout::{LayoutResult, TextMeasure};
use crate::style::locale::Direction;
use cache::take_cache_miss;
use measure::Pass;

type TextProbe = (Option<u32>, u32, u32);

pub struct LayoutEngine {
    drive: LayoutDrive,
    core_to_handle: SecondaryMap<slotmap::DefaultKey, NodeHandle>,
    synced_version: SecondaryMap<slotmap::DefaultKey, u32>,
    synced_text_sig: SecondaryMap<slotmap::DefaultKey, u64>,
    shared_text: Option<std::rc::Rc<crate::style::TextStyle>>,
    published_result: SecondaryMap<slotmap::DefaultKey, LayoutResult>,
    popup_offsets: SecondaryMap<slotmap::DefaultKey, (f32, f32)>,
    generation: u64,
    cached_key: Option<LayoutCacheKey>,
    reactive_layout: BTreeSet<slotmap::DefaultKey>,
    positions: SecondaryMap<slotmap::DefaultKey, (f32, f32)>,
    scopes: SecondaryMap<slotmap::DefaultKey, Shift>,
    scopes_generation: u64,
    text_sizes: SecondaryMap<slotmap::DefaultKey, TextProbe>,
    prepared: SecondaryMap<slotmap::DefaultKey, u64>,
    reactive_sigs: SecondaryMap<slotmap::DefaultKey, u64>,
    geometry_reactive: AHashSet<slotmap::DefaultKey>,
    steady: AHashSet<NodeId>,
    restyled: AHashSet<NodeId>,
    hidden: AHashSet<NodeId>,
    last_reactive_sig: u64,
    republished: bool,
}

impl core::fmt::Debug for LayoutEngine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LayoutEngine")
            .field("generation", &self.generation)
            .field("cached_key", &self.cached_key)
            .field("last_reactive_sig", &self.last_reactive_sig)
            .field("republished", &self.republished)
            .finish_non_exhaustive()
    }
}

impl LayoutEngine {
    #[must_use]
    pub fn new(measurer: Box<dyn TextMeasure>) -> Self {
        Self {
            drive: LayoutDrive::new(measurer),
            core_to_handle: SecondaryMap::new(),
            synced_version: SecondaryMap::new(),
            synced_text_sig: SecondaryMap::new(),
            shared_text: None,
            published_result: SecondaryMap::new(),
            popup_offsets: SecondaryMap::new(),
            generation: 0,
            cached_key: None,
            reactive_layout: BTreeSet::new(),
            positions: SecondaryMap::new(),
            scopes: SecondaryMap::new(),
            scopes_generation: 0,
            text_sizes: SecondaryMap::new(),
            prepared: SecondaryMap::new(),
            reactive_sigs: SecondaryMap::new(),
            geometry_reactive: AHashSet::new(),
            steady: AHashSet::new(),
            restyled: AHashSet::new(),
            hidden: AHashSet::new(),
            last_reactive_sig: 0,
            republished: false,
        }
    }

    pub(crate) const fn take_republished(&mut self) -> bool {
        std::mem::replace(&mut self.republished, false)
    }

    #[cfg(feature = "kittest")]
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
        let result = self
            .fresh(id)
            .or_else(|| self.published_result.get(id.key()).copied())?;
        Some(match self.popup_offsets.get(id.key()) {
            Some(&(dx, dy)) => result.translated(dx, dy),
            None => result,
        })
    }

    fn fresh(&self, id: NodeId) -> Option<LayoutResult> {
        let handle = self.handle(id).ok()?;
        let layout = self.drive.computed_box(handle)?;
        let shift = self
            .scopes
            .get(id.key())
            .copied()
            .or_else(|| self.published_result.get(id.key()).map(LayoutResult::shift));
        Some(
            LayoutResult::from_resolved_box(&layout, self.drive.clips_content(handle))
                .shifted(shift.unwrap_or_default()),
        )
    }

    pub fn sync(&mut self, cx: &Cx, root: NodeId) -> Result<(), Error> {
        self.gc(cx)?;
        let total = cx.len();
        self.drive.reserve(total);
        self.core_to_handle.set_capacity(total);
        self.synced_version.set_capacity(total);
        self.synced_text_sig.set_capacity(total);
        self.published_result.set_capacity(total);
        let root_text = TextStyle::default();
        self.sync_subtree(cx, root, InheritedText::new(&root_text, Direction::Ltr))
            .map(|_handles| ())
    }

    pub fn compute(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Constraint,
    ) -> Result<(), Error> {
        if cx.get(root).is_none() {
            return Err(Error::node_missing(root));
        }
        {
            let _children = crate::profile::span(crate::profile::Phase::LayoutChildren);
            cx.flush_reactive_children();
        }
        self.prepare_animations(cx, root);
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

        let resized = self.resized(cache_key);
        let mirror_current = dirty.is_empty() && self.same_inputs(cache_key);
        let kernel_len = dirty.len();
        self.seed_reactive_dirty(cx, &mut dirty, mirrored, mirror_current);
        attribution::note_dirty(cx, &dirty, kernel_len);
        let mut solved = SmallVec::<[NodeId; 8]>::new();
        if !resized && mirrored {
            solved.extend(self.translate_moved(cx, &mut dirty)?);
            if !solved.is_empty() && dirty.is_empty() {
                self.finish_translated(cx, (root, available), &solved, cache_key)?;
                return Ok(());
            }
        }
        let majority = resized || !mirrored || dirty.len() >= live_count.max(1) / 2;
        let pass = if mirror_current {
            Pass::Remeasure
        } else {
            Pass::changed(&dirty, majority, mirrored, structural)
        };
        solved.extend(self.solve_pass(cx, root, available, pass)?);
        self.measure_viewport_overlays(cx, root, available, &solved)?;
        self.finish(cx, cache_key);
        Ok(())
    }

    fn resized(&self, cache_key: LayoutCacheKey) -> bool {
        self.cached_key.is_some_and(|key| {
            key.available != cache_key.available
                || key.scale_bits != cache_key.scale_bits
                || key.fonts != cache_key.fonts
        })
    }

    fn same_inputs(&self, cache_key: LayoutCacheKey) -> bool {
        self.cached_key.is_some_and(|prev| {
            prev.cx_instance == cache_key.cx_instance
                && prev.root == cache_key.root
                && prev.input_signature == cache_key.input_signature
        })
    }

    fn finish_translated(
        &mut self,
        cx: &mut Cx,
        (root, available): (NodeId, Constraint),
        solved: &[NodeId],
        cache_key: LayoutCacheKey,
    ) -> Result<(), Error> {
        {
            let _overlays = crate::profile::span(crate::profile::Phase::LayoutOverlays);
            self.measure_viewport_overlays(cx, root, available, solved)?;
        }
        self.finish(cx, cache_key);
        Ok(())
    }

    fn finish(&mut self, cx: &mut Cx, cache_key: LayoutCacheKey) {
        let _finish = crate::profile::span(crate::profile::Phase::LayoutFinish);
        cx.clear_dirty();
        let unsized_labels: SmallVec<[NodeId; 8]> = self
            .reactive_layout
            .iter()
            .filter(|key| !self.text_sizes.contains_key(**key))
            .map(|key| NodeId::from_key(*key))
            .filter(|id| {
                cx.get(*id)
                    .is_some_and(|node| matches!(node.kind, crate::kernel::NodeType::Text { .. }))
            })
            .collect();
        for id in unsized_labels {
            self.remember_text_size(cx, id);
        }
        self.generation = self.generation.wrapping_add(1);
        let settled = !self.republished
            || self.reactive_layout.iter().all(|key| {
                cx.get(NodeId::from_key(*key)).is_some_and(|node| {
                    self.reactive_sigs.get(*key).copied() == Some(node.reactive_seed_signature())
                })
            });
        self.cached_key = settled.then_some(cache_key);
        let _sig = crate::profile::span(crate::profile::Phase::LayoutSig);
        self.last_reactive_sig = reactive_layout_signature(cx, &self.reactive_layout);
    }

    pub fn gc(&mut self, cx: &Cx) -> Result<(), Error> {
        let stale: Vec<_> = self
            .core_to_handle
            .keys()
            .map(NodeId::from_key)
            .filter(|id| cx.get(*id).is_none())
            .collect();
        for id in stale {
            if self.core_to_handle.contains_key(id.key()) {
                self.remove_mirror(id)?;
            }
        }
        Ok(())
    }

    fn remove_mirror(&mut self, id: NodeId) -> Result<(), Error> {
        let handle = self.handle(id)?;
        for h in self.drive.subtree_handles(handle)? {
            if let Some(core_id) = self.drive.kernel_for(h) {
                let _ = self.core_to_handle.remove(core_id.key());
                let _ = self.synced_version.remove(core_id.key());
                let _ = self.synced_text_sig.remove(core_id.key());
                let _ = self.published_result.remove(core_id.key());
                self.hidden.remove(&core_id);
                self.reactive_layout.remove(&core_id.key());
                self.geometry_reactive.remove(&core_id.key());
            }
        }
        self.drive.remove(handle)
    }

    fn reset_mirror(&mut self) {
        self.synced_version.clear();
        self.synced_text_sig.clear();
        self.published_result.clear();
        self.popup_offsets.clear();
        self.core_to_handle.clear();
        self.hidden.clear();
        self.reactive_layout.clear();
        self.geometry_reactive.clear();
        self.positions.clear();
        self.scopes.clear();
        self.text_sizes.clear();
        self.prepared.clear();
        self.reactive_sigs.clear();
        self.last_reactive_sig = 0;
    }

    fn track_reactive(&mut self, id: NodeId, node: &crate::kernel::Node) {
        if node.has_unversioned_layout() {
            self.reactive_layout.insert(id.key());
            self.reactive_sigs
                .insert(id.key(), node.reactive_seed_signature());
        } else {
            self.reactive_layout.remove(&id.key());
            self.reactive_sigs.remove(id.key());
        }
        if node.extension::<crate::layout::SizeReactive>().is_some()
            || node.extension::<crate::layout::BoundsReactive>().is_some()
        {
            self.geometry_reactive.insert(id.key());
        } else {
            self.geometry_reactive.remove(&id.key());
        }
        if let Some(pos) = node.extension::<crate::layout::ReactivePosition>() {
            self.positions.insert(id.key(), pos.get());
            pos.place();
        }
    }

    fn scope(&mut self, cx: &Cx, id: NodeId) -> Shift {
        if let Some(shift) = self.scopes.get(id.key()) {
            return *shift;
        }
        let under = cx
            .parent(id)
            .map_or_else(Shift::default, |parent| self.scope(cx, parent));
        let own = self
            .positions
            .contains_key(id.key())
            .then(|| cx.get(id))
            .flatten()
            .and_then(|node| node.extension::<crate::layout::ReactivePosition>())
            .map(|pos| pos.nest(under));
        let shift = own.unwrap_or(under);
        self.scopes.insert(id.key(), shift);
        shift
    }

    fn rescope(&mut self, cx: &Cx) {
        let generation = cx.mutation_generation();
        if self.scopes_generation != generation {
            self.scopes.clear();
            self.scopes_generation = generation;
        }
    }

    fn handle(&self, id: NodeId) -> Result<NodeHandle, Error> {
        self.core_to_handle
            .get(id.key())
            .copied()
            .ok_or_else(|| Error::node_missing(id))
    }
}

mod attribution;
mod cache;
mod children;
pub(crate) mod direction;
mod incremental;
mod measure;
mod overlays;
mod sync;
mod viewport;

pub(crate) use viewport::{Popup, PopupSide, ViewportOverlay};

fn note_full(cx: &Cx, root: NodeId, mirrored: bool, dirty: &[NodeId]) {
    crate::profile::count(crate::profile::Counter::LayoutFull);
    let reason = if !mirrored {
        crate::profile::Counter::LayoutFullUnmirrored
    } else if dirty.is_empty() {
        crate::profile::Counter::LayoutFullNoDirty
    } else {
        crate::profile::Counter::LayoutFullMajority
    };
    crate::profile::count(reason);
    attribution::note_full_root(cx, root, reason);
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
