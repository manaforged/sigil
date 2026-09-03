use ahash::AHashSet;
use sigil_hal::{Dim, Size};
use sigil_kernel::{Cx, NodeId, NodeType, TextContent};
use sigil_style::{StyleBinding, TextStyle};
use slotmap::SecondaryMap;
use smallvec::SmallVec;

use crate::cache_key::{
    AvailableKey, CacheDecision, LayoutCacheKey, has_unversioned_layout, layout_input_signature,
    reactive_layout_signature, reactive_seed_signature,
};
use crate::engine_drive::{LayoutDrive, NodeHandle};
use crate::forest::{AvailableSpace, SpaceConstraint};
use crate::style_data::{Dimension, Display};
use crate::text_geometry::TextStyleCapture;
use crate::text_style::{InheritedText, inherited_text_style, text_style_signature};
use crate::{LayoutError, LayoutResult, LayoutStyle, TextMeasure};

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
            .is_some_and(|key| key.cx_generation != cx.mutation_generation());

        let mirror_current = dirty.is_empty() && self.same_inputs(cache_key);
        self.seed_reactive_dirty(cx, &mut dirty, mirrored, mirror_current);
        if mirrored && self.translate_moved(cx, &mut dirty)? > 0 && dirty.is_empty() {
            self.finish(cx, cache_key);
            return Ok(());
        }
        let broadcast = !mirrored || dirty.is_empty() || dirty.len() >= live_count.max(1) / 2;
        if mirror_current {
            sigil_profile::count("layout.resize");
            self.remeasure_and_publish(cx, root, available)?;
        } else if broadcast {
            note_full(mirrored, &dirty);
            self.full_sync_and_measure(cx, root, available)?;
        } else {
            sigil_profile::count("layout.incr");
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
        self.cached_key = Some(cache_key);
        let _sig = sigil_profile::span("layout.sig");
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

    fn compute_mirror(&mut self, root: NodeId, available: Size) -> Result<(), LayoutError> {
        let root_handle = self.handle(root)?;
        self.drive
            .measure_with_constraints(root_handle, to_available_space(available))?;
        Ok(())
    }

    fn remeasure_and_publish(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        {
            let _solve = sigil_profile::span("layout.solve");
            self.compute_mirror(root, available)?;
        }
        let mut ids = SmallVec::<[_; 32]>::new();
        cx.visit_preorder(root, |id, _node| ids.push(id));
        let _attach = sigil_profile::span("layout.attach");
        self.attach_results(cx, ids)
    }

    fn full_sync_and_measure(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        let previous_ids: SmallVec<[NodeId; 32]> =
            self.core_to_handle.keys().map(NodeId::from_key).collect();
        {
            let _sync = sigil_profile::span("layout.sync");
            self.sync(cx, root)?;
        }
        {
            let _solve = sigil_profile::span("layout.solve");
            self.compute_mirror(root, available)?;
        }

        let mut ids = SmallVec::<[_; 32]>::new();
        cx.visit_preorder(root, |id, _node| ids.push(id));
        self.remove_stale_extensions(cx, previous_ids);
        let _attach = sigil_profile::span("layout.attach");
        self.attach_results(cx, ids)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "private helper splits the compute path; bundling these would obscure the broadcast/incremental branch"
    )]
    fn incremental_sync_and_measure(
        &mut self,
        cx: &mut Cx,
        dirty: &[NodeId],
        root: NodeId,
        available: Size,
        structural: bool,
    ) -> Result<(), LayoutError> {
        let roots = {
            let _dirty = sigil_profile::span("layout.dirty");
            self.dirty_roots(cx, dirty, root, structural)?
        };
        {
            let _solve = sigil_profile::span("layout.solve");
            self.solve_roots(&roots, root, available)?;
        }
        let _attach = sigil_profile::span("layout.attach");
        for subroot in roots {
            self.attach_subtree_results(cx, subroot)?;
        }
        Ok(())
    }

    fn dirty_roots(
        &mut self,
        cx: &mut Cx,
        dirty: &[NodeId],
        root: NodeId,
        structural: bool,
    ) -> Result<Vec<NodeId>, LayoutError> {
        let mut repositioned: Vec<NodeId> = dirty
            .iter()
            .copied()
            .filter(|id| self.position_changed(cx, *id))
            .collect();
        self.restyled.clear();
        self.apply_dirty_nodes(cx, dirty)?;
        repositioned.extend(self.restyled.iter().copied());
        for id in dirty.iter().copied() {
            self.clear_hidden(cx, id);
        }
        if structural && self.core_to_handle.len() > cx.len() {
            self.gc(cx)?;
        }
        let resized: Vec<NodeId> = dirty
            .iter()
            .copied()
            .filter(|id| !self.steady.contains(id) && !self.text_kept_size(cx, *id))
            .collect();
        sigil_profile::count_add("layout.dirty.n", dirty.len() as u64);
        sigil_profile::count_add("layout.resized.n", resized.len() as u64);
        let roots = self.containment_roots(cx, &resized, root, &repositioned);
        sigil_profile::count_add("layout.roots", roots.len() as u64);
        Ok(roots)
    }

    fn solve_roots(
        &mut self,
        roots: &[NodeId],
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        let origins: SmallVec<[(f32, f32); 4]> =
            roots.iter().map(|&r| self.published_origin(r)).collect();
        self.remeasure_roots(roots, root, available)?;
        for (&subroot, &(px, py)) in roots.iter().zip(origins.iter()) {
            let handle = self.handle(subroot)?;
            let solved = self
                .drive
                .stored_box(handle)
                .map_or((0.0, 0.0), |b| (b.x, b.y));
            self.drive
                .translate_subtree(handle, px - solved.0, py - solved.1)?;
        }
        Ok(())
    }

    fn text_kept_size(&mut self, cx: &Cx, id: NodeId) -> bool {
        let Some(node) = cx.get(id) else {
            return false;
        };
        if !matches!(node.kind, NodeType::Text { .. }) || !node.children().is_empty() {
            return false;
        }
        let Ok(handle) = self.handle(id) else {
            return false;
        };
        let unbounded = Size {
            width: Dim::MaxContent,
            height: Dim::MaxContent,
        };
        let saved = self.drive.stored_box(handle);
        if self
            .drive
            .measure_with_constraints(handle, to_available_space(unbounded))
            .is_err()
        {
            return false;
        }
        let measured = self.drive.computed_box(handle);
        if let Some(saved) = saved {
            drop(self.drive.write_box(handle, saved));
        }
        let Some(cb) = measured else {
            return false;
        };
        let size = (cb.w.to_bits(), cb.h.to_bits());
        self.text_sizes.insert(id.key(), size) == Some(size)
    }

    fn clear_hidden(&mut self, cx: &mut Cx, id: NodeId) {
        let hidden = self
            .handle(id)
            .ok()
            .and_then(|handle| self.drive.style_for(handle))
            .is_some_and(|style| matches!(style.display, Display::None));
        if !hidden {
            return;
        }
        let mut ids = SmallVec::<[_; 32]>::new();
        cx.visit_preorder(id, |node, _| ids.push(node));
        for node in ids {
            self.published_result.remove(node.key());
            if let Some(kernel) = cx.get_mut_untracked(node) {
                let _ = kernel.remove_extension::<LayoutResult>();
            }
        }
    }

    fn published_origin(&self, id: NodeId) -> (f32, f32) {
        self.published_result
            .get(id.key())
            .map_or((0.0, 0.0), |res| {
                let origin = res.rect().origin;
                (origin.x, origin.y)
            })
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

    fn sync_subtree(
        &mut self,
        cx: &Cx,
        id: NodeId,
        inherited: InheritedText<'_>,
    ) -> Result<SmallVec<[NodeHandle; 4]>, LayoutError> {
        let node = cx.get(id).ok_or(LayoutError::NodeMissing(id))?;
        self.track_reactive(id, node);
        let kernel_version = node.version();
        let children = node
            .children()
            .iter()
            .copied()
            .collect::<SmallVec<[_; 4]>>();
        if matches!(node.kind, NodeType::Fragment) {
            let mut out = SmallVec::<[NodeHandle; 4]>::new();
            for child in &children {
                out.extend(self.sync_subtree(cx, *child, inherited)?);
            }
            return Ok(out);
        }

        let binding = node.get_extension::<StyleBinding>();
        let layered = binding.and_then(|b| TextStyle::layered_over_binding(inherited.style, b));
        let effective = layered.as_ref().map_or(inherited, InheritedText::new);
        publish_text_style(node, effective.style);
        let text_sig_matches = self.synced_text_sig.get(id.key()).copied() == Some(effective.sig);

        let existing_handle = self.core_to_handle.get(id.key()).copied();
        let has_reactive = binding.is_some_and(StyleBinding::has_reactive)
            || node.get_extension::<crate::ReactivePosition>().is_some();
        let version_matches = !has_reactive
            && existing_handle.is_some()
            && self.synced_version.get(id.key()).copied() == Some(kernel_version)
            && text_sig_matches;
        if !version_matches {
            self.write_synced_node(id, node, inherited.style, kernel_version)?;
        }
        self.synced_text_sig.insert(id.key(), effective.sig);

        if binding.is_some_and(StyleBinding::hides) {
            let parent_handle = self.handle(id)?;
            self.drive.set_children(parent_handle, &[])?;
            let mut out = SmallVec::<[NodeHandle; 4]>::new();
            out.push(parent_handle);
            return Ok(out);
        }

        let mut child_handles = SmallVec::<[NodeHandle; 8]>::new();
        for child in &children {
            child_handles.extend(self.sync_subtree(cx, *child, effective)?);
        }
        let parent_handle = self.handle(id)?;
        self.drive.set_children(parent_handle, &child_handles)?;

        let mut out = SmallVec::<[NodeHandle; 4]>::new();
        out.push(parent_handle);
        Ok(out)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the argument list mirrors the upstream API contract"
    )]
    fn upsert_with_text(
        &mut self,
        id: NodeId,
        existing_handle: Option<NodeHandle>,
        intent: &LayoutStyle,
        text_content: Option<TextContent>,
        text_style: Option<TextStyle>,
        flags: crate::forest::TextFlags,
    ) -> Result<(), LayoutError> {
        if let Some(handle) = existing_handle {
            let style_matches = self
                .drive
                .style_for(handle)
                .is_some_and(|existing| existing == intent.data());
            if !style_matches {
                self.restyled.insert(id);
            }
            let text_matches = text_content.is_none() && self.drive.text_for(handle).is_none();
            if style_matches && text_matches {
                return Ok(());
            }
            let style = intent.data().clone();
            self.drive.set_style(handle, style)?;
            self.drive
                .set_text(handle, text_content, text_style, flags)?;
            return Ok(());
        }

        let style = intent.data().clone();
        let handle = self.drive.insert_with_kernel(id, style);
        if text_content.is_some() {
            self.drive
                .set_text(handle, text_content, text_style, flags)?;
        }
        self.core_to_handle.insert(id.key(), handle);
        Ok(())
    }

    fn apply_dirty_nodes(&mut self, cx: &Cx, dirty: &[NodeId]) -> Result<(), LayoutError> {
        for id in dirty.iter().copied() {
            if cx.get(id).is_some() {
                self.sync_one_node(cx, id)?;
            } else {
                self.remove_mirror_if_present(id)?;
            }
        }
        for id in dirty.iter().copied() {
            let Some(node) = cx.get(id) else {
                continue;
            };
            self.sync_child_list(cx, id, node)?;
        }
        Ok(())
    }

    fn sync_one_node(&mut self, cx: &Cx, id: NodeId) -> Result<(), LayoutError> {
        let node = cx.get(id).ok_or(LayoutError::NodeMissing(id))?;
        self.track_reactive(id, node);
        if matches!(node.kind, NodeType::Fragment) {
            return Ok(());
        }
        let inherited = inherited_text_style(cx, id);
        let (intent, text_content, text_style, flags) =
            Self::layout_inputs_for_node(node, &inherited);
        let existing = self.core_to_handle.get(id.key()).copied();
        self.upsert_with_text(id, existing, &intent, text_content, text_style, flags)?;
        self.synced_version.insert(id.key(), node.version());

        let layered = node
            .get_extension::<StyleBinding>()
            .and_then(|binding| TextStyle::layered_over_binding(&inherited, binding));
        let effective = layered.as_ref().unwrap_or(&inherited);
        publish_text_style(node, effective);
        let effective_sig = text_style_signature(effective);
        if self.synced_text_sig.get(id.key()).copied() != Some(effective_sig) {
            self.synced_text_sig.insert(id.key(), effective_sig);
            let children = node
                .children()
                .iter()
                .copied()
                .collect::<SmallVec<[_; 8]>>();
            for child in children {
                self.cascade_text_styles(cx, child, InheritedText::new(effective))?;
            }
        }
        Ok(())
    }

    fn cascade_text_styles(
        &mut self,
        cx: &Cx,
        id: NodeId,
        inherited: InheritedText<'_>,
    ) -> Result<(), LayoutError> {
        let Some(node) = cx.get(id) else {
            return Ok(());
        };
        let children = node
            .children()
            .iter()
            .copied()
            .collect::<SmallVec<[_; 8]>>();
        if matches!(node.kind, NodeType::Fragment) {
            for child in children {
                self.cascade_text_styles(cx, child, inherited)?;
            }
            return Ok(());
        }
        let layered = node
            .get_extension::<StyleBinding>()
            .and_then(|binding| TextStyle::layered_over_binding(inherited.style, binding));
        let effective = layered.as_ref().map_or(inherited, InheritedText::new);
        publish_text_style(node, effective.style);
        if self.synced_text_sig.get(id.key()).copied() == Some(effective.sig) {
            return Ok(());
        }
        self.synced_text_sig.insert(id.key(), effective.sig);
        if matches!(node.kind, NodeType::Text { .. }) {
            let (intent, text_content, text_style, flags) =
                Self::layout_inputs_for_node(node, inherited.style);
            let existing = self.core_to_handle.get(id.key()).copied();
            self.upsert_with_text(id, existing, &intent, text_content, text_style, flags)?;
        }
        for child in children {
            self.cascade_text_styles(cx, child, effective)?;
        }
        Ok(())
    }

    fn write_synced_node(
        &mut self,
        id: NodeId,
        node: &sigil_kernel::Node,
        inherited_text: &TextStyle,
        kernel_version: u32,
    ) -> Result<(), LayoutError> {
        let existing_handle = self.core_to_handle.get(id.key()).copied();
        let (intent, text_content, text_style, flags) =
            Self::layout_inputs_for_node(node, inherited_text);
        self.upsert_with_text(
            id,
            existing_handle,
            &intent,
            text_content,
            text_style,
            flags,
        )?;
        self.synced_version.insert(id.key(), kernel_version);
        Ok(())
    }

    fn layout_inputs_for_node(
        node: &sigil_kernel::Node,
        inherited_text: &TextStyle,
    ) -> (
        LayoutStyle,
        Option<TextContent>,
        Option<TextStyle>,
        crate::forest::TextFlags,
    ) {
        if matches!(node.kind, NodeType::Container)
            && node.get_extension::<StyleBinding>().is_none()
            && node.get_extension::<crate::ReactivePosition>().is_none()
        {
            let intent = node
                .get_extension::<LayoutStyle>()
                .cloned()
                .unwrap_or_default();
            return (intent, None, None, crate::forest::TextFlags::default());
        }
        let mut intent = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        let mut text_style = match &node.kind {
            NodeType::Text { .. } => Some(inherited_text.clone()),
            NodeType::Container | NodeType::Fragment | NodeType::Image { .. } => None,
        };
        if let NodeType::Image { source, .. } = &node.kind {
            intent = apply_intrinsic_image_size(intent, source.width(), source.height());
        }
        apply_style_overlay(node, &mut intent, text_style.as_mut());
        intent = apply_reactive_position(node, intent);
        let (text_content, flags) = match &node.kind {
            NodeType::Text {
                content,
                wrap,
                ellipsis,
            } => (
                Some(content.clone()),
                crate::forest::TextFlags {
                    wrap: *wrap,
                    ellipsis: *ellipsis,
                },
            ),
            NodeType::Container | NodeType::Fragment | NodeType::Image { .. } => {
                (None, crate::forest::TextFlags::default())
            }
        };
        (intent, text_content, text_style, flags)
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

    fn track_reactive(&mut self, id: NodeId, node: &sigil_kernel::Node) {
        if has_unversioned_layout(node) {
            self.reactive_layout.insert(id.key());
        } else {
            self.reactive_layout.remove(&id.key());
        }
        if let Some(pos) = node.get_extension::<crate::ReactivePosition>() {
            self.positions.insert(id.key(), pos.get());
        }
    }

    fn position_changed(&self, cx: &Cx, id: NodeId) -> bool {
        cx.get(id)
            .and_then(|node| node.get_extension::<crate::ReactivePosition>())
            .is_some_and(|pos| self.positions.get(id.key()).copied() != Some(pos.get()))
    }

    fn moved_only(&self, cx: &Cx, id: NodeId) -> bool {
        let Some(node) = cx.get(id) else {
            return false;
        };
        let Some(pos) = node.get_extension::<crate::ReactivePosition>() else {
            return false;
        };
        self.positions
            .get(id.key())
            .is_some_and(|prior| *prior != pos.get())
            && self.synced_version.get(id.key()).copied() == Some(node.version())
    }

    fn translate_moved(
        &mut self,
        cx: &mut Cx,
        dirty: &mut Vec<NodeId>,
    ) -> Result<usize, LayoutError> {
        let moved: Vec<NodeId> = dirty
            .iter()
            .copied()
            .filter(|id| self.moved_only(cx, *id))
            .collect();
        dirty.retain(|id| !moved.contains(id));
        for id in moved.iter().copied() {
            sigil_profile::count("layout.translate");
            self.translate(cx, id)?;
        }
        Ok(moved.len())
    }

    fn translate(&mut self, cx: &mut Cx, id: NodeId) -> Result<(), LayoutError> {
        let node = cx.get(id).ok_or(LayoutError::NodeMissing(id))?;
        let Some(next) = node
            .get_extension::<crate::ReactivePosition>()
            .map(crate::ReactivePosition::get)
        else {
            return Ok(());
        };
        let prior = self.positions.get(id.key()).copied().unwrap_or(next);
        if prior == next {
            return Ok(());
        }
        self.sync_one_node(cx, id)?;
        let handle = self.handle(id)?;
        self.drive
            .translate_subtree(handle, next.0 - prior.0, next.1 - prior.1)?;
        self.positions.insert(id.key(), next);
        self.attach_subtree_results(cx, id)
    }

    fn remove_mirror_if_present(&mut self, id: NodeId) -> Result<(), LayoutError> {
        if self.core_to_handle.contains_key(id.key()) {
            self.remove_mirror(id)?;
        }
        Ok(())
    }

    fn sync_child_list(
        &mut self,
        cx: &Cx,
        id: NodeId,
        node: &sigil_kernel::Node,
    ) -> Result<(), LayoutError> {
        let parent = if matches!(node.kind, NodeType::Fragment) {
            self.nearest_mirrored_ancestor(cx, id)
        } else {
            Some(id)
        };
        let Some(parent) = parent else {
            return Ok(());
        };
        let Some(parent_node) = cx.get(parent) else {
            return Ok(());
        };
        if parent_node
            .get_extension::<StyleBinding>()
            .is_some_and(StyleBinding::hides)
        {
            self.drive.set_children(self.handle(parent)?, &[])?;
            return Ok(());
        }
        let handles = self.flatten_child_handles(cx, parent_node.children())?;
        self.drive.set_children(self.handle(parent)?, &handles)
    }

    #[allow(
        clippy::unused_self,
        reason = "kept as method for symmetry with sibling sync helpers; will gain &self use when subtree mirror tracking lands"
    )]
    fn nearest_mirrored_ancestor(&self, cx: &Cx, id: NodeId) -> Option<NodeId> {
        let mut current = cx.parent(id);
        while let Some(parent) = current {
            let node = cx.get(parent)?;
            if !matches!(node.kind, NodeType::Fragment) {
                return Some(parent);
            }
            current = cx.parent(parent);
        }
        None
    }

    fn flatten_child_handles(
        &mut self,
        cx: &Cx,
        children: &[NodeId],
    ) -> Result<Vec<NodeHandle>, LayoutError> {
        let mut handles = Vec::new();
        for child in children {
            let node = cx.get(*child).ok_or(LayoutError::NodeMissing(*child))?;
            if matches!(node.kind, NodeType::Fragment) {
                handles.extend(self.flatten_child_handles(cx, node.children())?);
            } else if let Some(handle) = self.core_to_handle.get(child.key()).copied() {
                handles.push(handle);
            } else {
                let text = inherited_text_style(cx, *child);
                let inherited = InheritedText::new(&text);
                handles.extend(self.sync_subtree(cx, *child, inherited)?);
            }
        }
        Ok(handles)
    }

    #[allow(
        clippy::unused_self,
        reason = "kept as method for symmetry with sibling helpers; will gain &self use in incremental sync follow-up"
    )]
    fn containment_roots(
        &self,
        cx: &Cx,
        dirty: &[NodeId],
        root: NodeId,
        repositioned: &[NodeId],
    ) -> Vec<NodeId> {
        let mut seen = AHashSet::with_capacity(dirty.len());
        let mut roots = Vec::with_capacity(dirty.len());
        for id in dirty.iter().copied() {
            let start = if Self::has_definite_size(cx, id) && !repositioned.contains(&id) {
                id
            } else {
                cx.parent(id).unwrap_or(id)
            };
            let containment = Self::find_containment_root(cx, start, root);
            if seen.insert(containment) {
                roots.push(containment);
            }
        }
        roots.retain(|id| !Self::has_ancestor_in(cx, &seen, *id));
        roots
    }

    fn has_ancestor_in(cx: &Cx, set: &AHashSet<NodeId>, id: NodeId) -> bool {
        let mut current = cx.parent(id);
        while let Some(parent) = current {
            if set.contains(&parent) {
                return true;
            }
            current = cx.parent(parent);
        }
        false
    }

    fn find_containment_root(cx: &Cx, start: NodeId, root: NodeId) -> NodeId {
        let mut current = start;
        loop {
            if current == root || Self::has_definite_size(cx, current) {
                return current;
            }
            let Some(parent) = cx.parent(current) else {
                return root;
            };
            current = parent;
        }
    }

    fn has_definite_size(cx: &Cx, id: NodeId) -> bool {
        let Some(node) = cx.get(id) else {
            return false;
        };
        let style = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        let definite = |dim: Dimension| {
            matches!(
                dim,
                Dimension::Px(_) | Dimension::Percent(_) | Dimension::Fill
            )
        };
        definite(style.data().size.width) && definite(style.data().size.height)
    }

    fn remeasure_roots(
        &mut self,
        roots: &[NodeId],
        tree_root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        self.drive.begin_measure_frame();
        let space = to_available_space(available);
        for root in roots {
            let handle = self.handle(*root)?;
            let assigned = (*root != tree_root)
                .then(|| self.published_result.get(root.key()).copied())
                .flatten();
            match assigned {
                Some(published) => {
                    let rect = published.rect();
                    self.drive
                        .measure_assigned(handle, space, rect.width, rect.height)?;
                }
                None => self.drive.measure_keep_frame(handle, space)?,
            }
        }
        Ok(())
    }

    fn attach_subtree_results(&mut self, cx: &mut Cx, root: NodeId) -> Result<(), LayoutError> {
        let mut ids = SmallVec::<[_; 32]>::new();
        cx.visit_preorder(root, |id, _node| ids.push(id));
        self.attach_results(cx, ids)
    }

    fn attach_results(
        &mut self,
        cx: &mut Cx,
        ids: SmallVec<[NodeId; 32]>,
    ) -> Result<(), LayoutError> {
        for id in ids {
            let Some(result) = self.fresh(id) else {
                continue;
            };
            if self.published_result.get(id.key()).copied() == Some(result) {
                continue;
            }
            let bindings = {
                let node = cx
                    .get_mut_untracked(id)
                    .ok_or(LayoutError::NodeMissing(id))?;
                let _ = node.attach_untracked(result);
                let size = node.get_extension::<crate::SizeReactive>().copied();
                let bounds = node.get_extension::<crate::BoundsReactive>().copied();
                (size, bounds)
            };
            let (size_binding, bounds_binding) = bindings;
            if size_binding.is_some() || bounds_binding.is_some() {
                let rect = result.rect();
                if let Some(sr) = size_binding {
                    sr.publish((rect.width, rect.height));
                }
                if let Some(br) = bounds_binding {
                    br.publish((rect.origin.x, rect.origin.y, rect.width, rect.height));
                }
            }
            self.published_result.insert(id.key(), result);
        }
        Ok(())
    }

    fn remove_stale_extensions(&mut self, cx: &mut Cx, previous_ids: SmallVec<[NodeId; 32]>) {
        for id in previous_ids {
            let still_fresh = self
                .core_to_handle
                .get(id.key())
                .copied()
                .is_some_and(|handle| self.drive.computed_box(handle).is_some());
            if still_fresh {
                continue;
            }
            self.published_result.remove(id.key());
            if let Some(node) = cx.get_mut_untracked(id) {
                if node.get_extension::<LayoutResult>().is_none() {
                    continue;
                }
                let _ = node.remove_extension::<LayoutResult>();
            }
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
            sigil_profile::count("layout.hit");
            cx.clear_dirty();
            None
        }
        CacheDecision::Miss(key) => {
            sigil_profile::count("layout.miss");
            Some(key)
        }
    }
}

fn publish_text_style(node: &sigil_kernel::Node, style: &TextStyle) {
    if let Some(capture) = node.get_extension::<TextStyleCapture>() {
        capture.publish(style);
    }
}

const fn to_available_space(size: Size) -> AvailableSpace {
    AvailableSpace {
        w: to_space_constraint(size.width),
        h: to_space_constraint(size.height),
    }
}

const fn to_space_constraint(dim: Dim) -> SpaceConstraint {
    match dim {
        Dim::Fixed(value) => SpaceConstraint::Definite(value),
        Dim::MinContent => SpaceConstraint::MinContent,
        Dim::MaxContent => SpaceConstraint::MaxContent,
    }
}

mod overlays;

use overlays::{apply_intrinsic_image_size, apply_reactive_position, apply_style_overlay};
fn animated(node: &sigil_kernel::Node) -> bool {
    node.get_extension::<StyleBinding>().is_some_and(|binding| {
        binding.animator_overlay().is_some() || binding.state_overlay().is_some()
    })
}

fn note_full(mirrored: bool, dirty: &[NodeId]) {
    sigil_profile::count("layout.full");
    sigil_profile::count(if !mirrored {
        "layout.full.unmirrored"
    } else if dirty.is_empty() {
        "layout.full.nodirty"
    } else {
        "layout.full.majority"
    });
}
