//! Layout engine and `sigil-kernel` synchronization.

use ahash::AHashSet;
use sigil_hal::{Dim, Size};
use sigil_kernel::{Cx, NodeId, NodeType, TextContent};
use sigil_style::{StyleBinding, StyleEngine, TextStyle};
use slotmap::SecondaryMap;
use smallvec::SmallVec;

use crate::cache_key::{
    AvailableKey, CacheDecision, LayoutCacheKey, has_unversioned_layout, layout_input_signature,
    reactive_layout_signature,
};
use crate::engine_drive::{LayoutDrive, NodeHandle};
use crate::forest::{AvailableSpace, SpaceConstraint};
use crate::style_adapter::apply_style_atoms;
use crate::style_data::Dimension;
use crate::text_geometry::TextStyleCapture;
use crate::text_style::{
    InheritedText, apply_text_binding, inherited_text_style, text_style_signature, theme_for,
};
use crate::{LayoutError, LayoutResult, LayoutStyle, TextMeasure};

/// Owned layout engine mirroring a `sigil-kernel` subtree into [`LayoutDrive`].
pub struct LayoutEngine {
    drive: LayoutDrive,
    /// `NodeId → NodeHandle` mirror map. Slotmap-keyed (not hashed) so
    /// the hot sync path is O(1) array indexing instead of an `AHashMap`
    /// lookup per node.
    core_to_handle: SecondaryMap<slotmap::DefaultKey, NodeHandle>,
    /// Last kernel-side `Node::version()` we observed per node. The
    /// sync hot path peeks the kernel version *before* doing any
    /// `get_extension` / `clone` / `set_style` work; when it matches,
    /// the per-node payload is provably unchanged and we skip the
    /// entire input-extraction + write path. Vector ε1 — eliminates
    /// the two `AHashMap` lookups + `LayoutStyle` clone that were 65%
    /// of the broadcast frame post-vector-δ.
    synced_version: SecondaryMap<slotmap::DefaultKey, u32>,
    /// Signature of the *effective* typography (inherited ⊕ own atoms)
    /// each node saw at last sync. A node's own `Node::version` cannot
    /// observe ancestor typography changes, so the sync fast-path
    /// additionally requires this signature to match (DR-056
    /// invalidation). Doubles as the prune key for the incremental
    /// cascade: an unchanged signature proves the whole subtree's
    /// inherited input is unchanged.
    synced_text_sig: SecondaryMap<slotmap::DefaultKey, u64>,
    /// Last `LayoutResult` we attached to each kernel node. Read via
    /// O(1) slotmap indexing instead of the kernel's
    /// `AHashMap<TypeId, Box<dyn Any>>` extension lookup. Comparing
    /// new vs cached lets `attach_results` skip nodes whose box did
    /// not move — eliminates the per-node `get_extension::<LayoutResult>()`
    /// `AHashMap` probe that was ~80 ns × 101k = ~8 ms per broadcast.
    published_result: SecondaryMap<slotmap::DefaultKey, LayoutResult>,
    generation: u64,
    cached_key: Option<LayoutCacheKey>,
    /// Kernel nodes whose layout input can change without a version bump.
    reactive_layout: AHashSet<slotmap::DefaultKey>,
    last_reactive_sig: u64,
}

impl LayoutEngine {
    /// Create a layout engine with the given text measurer.
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
            last_reactive_sig: 0,
        }
    }

    /// Return the number of nodes currently mirrored into the owned drive.
    #[must_use]
    pub fn mirrored_node_count(&self) -> usize {
        self.core_to_handle.len()
    }

    /// Monotonic counter bumped on every successful layout recompute.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Return the most recently computed result for a node. Reads
    /// directly from the drive's forest — no parallel result map.
    #[must_use]
    pub fn result(&self, id: NodeId) -> Option<LayoutResult> {
        let handle = self.core_to_handle.get(id.key()).copied()?;
        let layout = self.drive.computed_box(handle)?;
        let clips = self.drive.clips_content(handle);
        Some(LayoutResult::from_resolved_box(&layout, clips))
    }

    /// Return the parent's resolved content-box size for `id` from the
    /// most recent layout pass.
    ///
    /// Returns `None` when `id` has no parent in `cx`, when the parent
    /// is not mirrored, or when no layout has been computed yet.
    ///
    /// Widgets that need to size against their containing block can
    /// query this after a layout pass instead of being told the size
    /// at build time. The preferred build-time path is
    /// [`crate::Dimension::Fill`], which the solvers resolve to the
    /// containing block on the corresponding axis.
    #[must_use]
    pub fn parent_content_box(&self, cx: &Cx, id: NodeId) -> Option<Size> {
        let parent = cx.parent(id)?;
        let result = self.result(parent)?;
        let rect = result.rect();
        Some(Size::fixed(rect.width, rect.height))
    }

    /// Mirror a `sigil-kernel` subtree into the private owned layout drive.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::NodeMissing`] when `root` is stale or a
    /// child mapping is absent.
    pub fn sync(&mut self, cx: &Cx, root: NodeId) -> Result<(), LayoutError> {
        let root_text = TextStyle::default();
        self.sync_subtree(cx, root, InheritedText::new(&root_text))
            .map(|_handles| ())
    }

    /// Compute layout for a `sigil-kernel` subtree and attach results back to nodes.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::NodeMissing`] when `root` or a computed
    /// node is stale, and returns [`LayoutError::LayoutInvalid`] when
    /// the owned drive rejects a stale mirror handle.
    pub fn compute(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        if cx.get(root).is_none() {
            return Err(LayoutError::NodeMissing(root));
        }
        // Drain any deferred reactive-children rebuilds BEFORE the
        // cache-key check. Each rebuild calls `Cx::remove` + `Cx::add_child`,
        // which marks the affected nodes dirty via `mark_dirty`. The
        // cache-key resolution below reads `cx.dirty()` and
        // `cx.mutation_generation()`, so the drain must happen first —
        // otherwise the engine would see a clean dirty-set, return
        // `CacheDecision::Hit`, and miss the structural change entirely.
        cx.flush_reactive_children();
        let Some(cache_key) = take_cache_miss(cx, self.resolve_cache_key(cx, root, available))
        else {
            return Ok(());
        };
        // ε-attach safety: when the Cx instance changes (caller
        // swapped in a fresh tree), the slotmap keys we cached against
        // belong to a different generation. Reuse would false-positive
        // the version + result equality checks. Drop them.
        let cx_instance_changed = self
            .cached_key
            .is_some_and(|prev| prev.cx_instance != cache_key.cx_instance);
        if cx_instance_changed {
            self.synced_version.clear();
            self.synced_text_sig.clear();
            self.published_result.clear();
            self.core_to_handle.clear();
            self.reactive_layout.clear();
            self.last_reactive_sig = 0;
        }

        self.drive.begin_frame();
        let mut dirty = cx.dirty().collect::<Vec<_>>();
        let live_count = cx.len();
        // The incremental path resolves handles for non-dirty kept nodes;
        // an engine that never mirrored this tree has none — full-sync.
        let mirrored = self.core_to_handle.contains_key(root.key());
        let structural = self
            .cached_key
            .is_some_and(|key| key.cx_generation != cx.mutation_generation());

        // Resize fast-path: on a MISS with an empty dirty set and an input
        // signature identical to the last compute, only `available` changed
        // (a viewport resize). The mirror is already current — skip the
        // O(nodes) `sync` walk and only re-solve + re-publish.
        let mirror_current = dirty.is_empty()
            && self.cached_key.is_some_and(|prev| {
                prev.cx_instance == cache_key.cx_instance
                    && prev.root == cache_key.root
                    && prev.input_signature == cache_key.input_signature
            });
        self.seed_reactive_dirty(&mut dirty, mirrored, mirror_current);
        let broadcast = !mirrored || dirty.is_empty() || dirty.len() >= live_count.max(1) / 2;
        if mirror_current {
            sigil_profile::count("layout.resize");
            self.remeasure_and_publish(cx, root, available)?;
        } else if broadcast {
            sigil_profile::count("layout.full");
            self.full_sync_and_measure(cx, root, available)?;
        } else {
            sigil_profile::count("layout.incr");
            self.incremental_sync_and_measure(cx, &dirty, root, available, structural)?;
        }

        cx.clear_dirty();
        self.generation = self.generation.wrapping_add(1);
        // Store the key as-is: no post-work signature re-hash. Layout output
        // is attached UNTRACKED (`attach_untracked`), so nothing this compute
        // did moved a version the signature folds in — the input signature at
        // the end of the frame equals the one already in `cache_key`. So a
        // later CLEAN frame that re-hashes and matches this value is still an
        // immediate cache HIT, without paying an O(nodes) re-walk here.
        // (On a dirty-driven miss `cache_key.input_signature` is the carried
        // prior value; the first clean frame after the dirty burst re-hashes
        // and misses once, then hits — never a false hit.)
        self.cached_key = Some(cache_key);
        self.last_reactive_sig = reactive_layout_signature(cx, &self.reactive_layout);
        Ok(())
    }

    /// Remove mirrored layout nodes for `sigil-kernel` nodes no longer present.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::LayoutInvalid`] when the owned drive sees a stale handle.
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

    /// Re-solve the existing (already-current) mirror against new constraints
    /// and republish results, skipping the kernel->drive `sync`. Used by the
    /// resize fast-path when the kernel tree is unchanged and only `available`
    /// differs. No node was added or removed, so there are no stale extensions
    /// to reap and the id set is the live preorder walk.
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
        self.apply_dirty_nodes(cx, dirty)?;
        // `gc` scans every mirror to drop the ones whose kernel node is gone
        // — an O(nodes) walk. `structural` (the generation changed) is far
        // broader than "a node was removed": a plain style mutation bumps the
        // generation too, and on a 100k-node tree that made every restyle pay
        // a full stale-mirror scan that found nothing. After `apply_dirty_nodes`
        // has synced the dirty set (adding new mirrors, dropping dirty-gone
        // ones), a lingering stale mirror can only exist when the mirror count
        // exceeds the live kernel count — an O(1) check that gates the scan to
        // the frames that actually removed a subtree.
        if structural && self.core_to_handle.len() > cx.len() {
            self.gc(cx)?;
        }
        let roots = self.containment_roots(cx, dirty, root);
        // `remeasure_roots` re-solves each root as an isolated layout root,
        // which anchors it at (0,0) — size-correct but position-destroying for
        // any root that is not the tree root. Capture each root's true in-tree
        // origin (still set by its unchanged parent) BEFORE the remeasure, then
        // shift its subtree's DRIVE boxes back so every downstream reader —
        // paint (attached `LayoutResult`), hit-testing and pointer dispatch
        // (both via `result()` → the drive) — agrees on one absolute position
        // (GLM review MAJOR-1). The tree root's origin is (0,0), a no-op shift.
        let origins: SmallVec<[(f32, f32); 4]> =
            roots.iter().map(|&r| self.published_origin(r)).collect();
        {
            let _solve = sigil_profile::span("layout.solve");
            self.remeasure_roots(&roots, available)?;
            for (&subroot, &(dx, dy)) in roots.iter().zip(origins.iter()) {
                let handle = self.handle(subroot)?;
                self.drive.translate_subtree(handle, dx, dy)?;
            }
        }
        let _attach = sigil_profile::span("layout.attach");
        for subroot in roots {
            self.attach_subtree_results(cx, subroot)?;
        }
        Ok(())
    }

    /// The root-coordinate top-left a node was last published at, or `(0, 0)`
    /// if it has no published result yet. The incremental remeasure path reads
    /// this before zeroing the root, to restore its position afterward.
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
        // Specific dirties are a miss (a version the hash folds in
        // changed). A clean frame with a matching generation and
        // matching unversioned-input stamp is a hit — no tree walk.
        // `mark_mutated` (`dirty_all`) bumps generation, so it still
        // re-hashes.
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

    /// Sync one kernel node and return the handles the caller should
    /// register as children. Fragments flatten into their parent.
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

        // DR-056 typography inheritance: layer this node's typography
        // atoms over the inherited style. `layered` is `None` for the
        // common no-typography-atoms case, so containers pass the
        // inherited value through by reference without cloning.
        let binding = node.get_extension::<StyleBinding>();
        let layered = binding.and_then(|b| TextStyle::layered_over_binding(inherited.style, b));
        let effective = layered.as_ref().map_or(inherited, InheritedText::new);
        publish_text_style(node, effective.style);
        let text_sig_matches = self.synced_text_sig.get(id.key()).copied() == Some(effective.sig);

        // ε1 fast-path: when the kernel-side `Node::version` matches
        // the version we cached on the last sync, every observable
        // payload (style binding, layout intent, text content, a11y,
        // child list) is provably unchanged. Skip `layout_inputs_for_node`
        // (two AHashMap lookups + LayoutStyle clone) and `upsert_with_text`
        // entirely. Children still recurse — their versions are checked
        // independently. `set_children` will cheap-out (δ9) when its
        // child list is byte-identical. This single lookup also feeds
        // `upsert_with_text`'s existing-handle branch on a miss.
        let existing_handle = self.core_to_handle.get(id.key()).copied();
        // Reactive style slices and reactive position mutate through
        // interior-mutable channels that bypass `Node::version`.
        // `has_reactive` stays true when the current slice is empty
        // (`Display::None` → `[]` still re-syncs). Downstream
        // style-equality still cheap-outs when nothing changed.
        let has_reactive = binding.is_some_and(StyleBinding::has_reactive)
            || node.get_extension::<crate::ReactivePosition>().is_some();
        // DR-056: the version fast-path additionally requires the
        // effective-typography signature to match — a node's own
        // version cannot see an ancestor's `text-*` change, but the
        // carried signature can.
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

    #[allow(clippy::too_many_arguments)]
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
            // Cheap-out: if both the style and the text payload already
            // equal what the kernel just produced, skip the
            // clone-and-write entirely. Profile (wide_tree, 101k
            // nodes) showed `sync` was 52% of the broadcast frame
            // even though styles were identical iteration-to-iteration.
            let style_matches = self
                .drive
                .style_for(handle)
                .is_some_and(|existing| existing == intent.data());
            // Text equality is conservative: we only fast-path the
            // `(None, None)` case because `TextContent` doesn't
            // implement `PartialEq` (the reactive variant carries a
            // closure). When either side has text, fall through to
            // the write path — text mutations are rare relative to
            // style mutations.
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
        // DR-056: the incremental path has no top-down carry, so the
        // inherited typography is resolved by ancestor walk.
        let inherited = inherited_text_style(cx, id);
        let (intent, text_content, text_style, flags) =
            Self::layout_inputs_for_node(node, &inherited);
        let existing = self.core_to_handle.get(id.key()).copied();
        self.upsert_with_text(id, existing, &intent, text_content, text_style, flags)?;

        // If this node's *effective* typography changed, descendants
        // measured against the old value are stale even though their
        // own versions are unchanged. Cascade re-derivation, pruning
        // at any subtree whose stored signature already matches.
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

    /// Re-derive the text inputs of descendants whose inherited
    /// typography changed (DR-056 incremental cascade). Containers only
    /// transmit; text nodes re-measure. A subtree whose stored
    /// signature matches the incoming one is provably unchanged and is
    /// pruned without descending.
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

    /// Extract a node's layout inputs and write them into the mirror,
    /// recording the version that produced them. Split out of
    /// `sync_subtree` to keep that hot path within the line budget.
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
        // A raster image is a leaf sized to its pixels unless the author
        // pinned a dimension: seed any `Auto` axis with the intrinsic
        // extent so an unsized image lays out at its natural size while a
        // `.width()`/`.height()` override (already folded into `intent`
        // above) still wins.
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

    /// Wrapping `text(derived)` does not fill `Cx::dirty`. The miss is
    /// `reactive_layout_signature`. Seed those nodes so incr remasures
    /// them instead of `layout.full`.
    fn seed_reactive_dirty(&self, dirty: &mut Vec<NodeId>, mirrored: bool, mirror_current: bool) {
        if mirrored && dirty.is_empty() && !mirror_current {
            dirty.extend(self.reactive_layout.iter().copied().map(NodeId::from_key));
        }
    }

    fn track_reactive(&mut self, id: NodeId, node: &sigil_kernel::Node) {
        if has_unversioned_layout(node) {
            self.reactive_layout.insert(id.key());
        } else {
            self.reactive_layout.remove(&id.key());
        }
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
        &self,
        cx: &Cx,
        children: &[NodeId],
    ) -> Result<Vec<NodeHandle>, LayoutError> {
        let mut handles = Vec::new();
        for child in children {
            let node = cx.get(*child).ok_or(LayoutError::NodeMissing(*child))?;
            if matches!(node.kind, NodeType::Fragment) {
                handles.extend(self.flatten_child_handles(cx, node.children())?);
            } else {
                handles.push(self.handle(*child)?);
            }
        }
        Ok(handles)
    }

    #[allow(
        clippy::unused_self,
        reason = "kept as method for symmetry with sibling helpers; will gain &self use in incremental sync follow-up"
    )]
    fn containment_roots(&self, cx: &Cx, dirty: &[NodeId], root: NodeId) -> Vec<NodeId> {
        let mut seen = AHashSet::with_capacity(dirty.len());
        let mut roots = Vec::with_capacity(dirty.len());
        for id in dirty.iter().copied() {
            let start = cx.parent(id).unwrap_or(id);
            let containment = Self::find_containment_root(cx, start, root);
            if seen.insert(containment) {
                roots.push(containment);
            }
        }
        // Drop any containment root nested under another. `remeasure_roots`
        // solves each root as an isolated layout root anchored at (0,0); a root
        // that is a descendant of another root would have its real in-tree
        // position clobbered to the origin (the editor's tall scrolled body
        // escaping to the top-left, painting over the explorer). The ancestor's
        // remeasure already covers the descendant, so keep only ancestor-most
        // roots.
        roots.retain(|id| !Self::has_ancestor_in(cx, &seen, *id));
        roots
    }

    /// True when any *other* node in `set` is a proper ancestor of `id`.
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
            if current == root || Self::has_fixed_size(cx, current) {
                return current;
            }
            let Some(parent) = cx.parent(current) else {
                return root;
            };
            current = parent;
        }
    }

    fn has_fixed_size(cx: &Cx, id: NodeId) -> bool {
        let Some(node) = cx.get(id) else {
            return false;
        };
        let style = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        matches!(style.data().size.width, Dimension::Px(_))
            && matches!(style.data().size.height, Dimension::Px(_))
    }

    fn remeasure_roots(&mut self, roots: &[NodeId], available: Size) -> Result<(), LayoutError> {
        for root in roots {
            let handle = self.handle(*root)?;
            self.drive
                .measure_with_constraints(handle, to_available_space(available))?;
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
            // Drive boxes are already in absolute coordinates here: the
            // incremental path shifted the isolated-remeasure subtree back to
            // its origin before this attach (see `incremental_sync_and_measure`).
            let Some(result) = self.result(id) else {
                continue;
            };
            // ε-attach: compare against our own slotmap-indexed
            // `published_result` cache instead of going through the
            // kernel node's `AHashMap<TypeId, Box<dyn Any>>` extension
            // lookup. Slotmap is O(1) array indexing — a single load
            // vs the AHashMap's hash + probe + downcast (~5× cheaper
            // per node, and at 101k nodes that's ~8 ms saved per
            // broadcast).
            if self.published_result.get(id.key()).copied() == Some(result) {
                continue;
            }
            // Take the size/bounds bindings (if any) and DROP the `cx` borrow before
            // publishing: `Signal::set` runs dependent effects synchronously, and a
            // reactive consumer's `children_reactive` rebuild must be free to read
            // `cx` without colliding with this mutable borrow.
            let bindings = {
                // Layout OUTPUT: attach untracked so writing the resolved box
                // back onto the node neither re-dirties it nor bumps the input
                // version — otherwise every laid-out node would invalidate the
                // input signature its own layout produced, forcing a full
                // O(nodes) re-hash each frame and defeating the cache.
                let node = cx
                    .get_mut_untracked(id)
                    .ok_or(LayoutError::NodeMissing(id))?;
                let _ = node.attach_untracked(result);
                let size = node.get_extension::<crate::SizeReactive>().copied();
                let bounds = node.get_extension::<crate::BoundsReactive>().copied();
                (size, bounds)
            };
            // Publish to the bindings. Only reached when the box changed (the
            // dedup above), and each `publish` dedups its own value (so a
            // position-only change is a no-op for the size binding), so this
            // is free on steady frames. The write is legal in the Layout
            // phase (only Paint forbids writes). The dependent effect runs
            // inline here, but its tree mutation is enqueued and flushed by
            // the next `compute`; `frame_dirty` is set so that next frame
            // applies the new box.
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

    fn remove_stale_extensions(&self, cx: &mut Cx, previous_ids: SmallVec<[NodeId; 32]>) {
        for id in previous_ids {
            // A node still has a fresh result if its mirror handle is
            // still registered AND the drive reports a current-frame
            // box for it. Anything else is stale and loses its
            // `LayoutResult` extension.
            let still_fresh = self
                .core_to_handle
                .get(id.key())
                .copied()
                .is_some_and(|handle| self.drive.computed_box(handle).is_some());
            if still_fresh {
                continue;
            }
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

/// Reactive absolute position wins over any static inset.
fn apply_reactive_position(node: &sigil_kernel::Node, intent: LayoutStyle) -> LayoutStyle {
    let Some(pos) = node.get_extension::<crate::ReactivePosition>() else {
        return intent;
    };
    let (x, y) = pos.get();
    intent
        .position(crate::style_data::Position::Absolute)
        .inset_left(x)
        .inset_top(y)
}

/// Overlay class / Style atoms onto the box. Authored `LayoutStyle`
/// padding wins when it is not the zero default.
fn apply_style_overlay(
    node: &sigil_kernel::Node,
    intent: &mut LayoutStyle,
    text_style: Option<&mut TextStyle>,
) {
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return;
    };
    let pad = intent.data().padding;
    let keep_pad =
        pad != crate::style_data::Rect::splat(crate::style_data::LengthPercentage::Px(0.0));
    apply_style_atoms(binding.static_slice(), intent);
    apply_style_atoms(binding.dynamic_slice(), intent);
    for reactive in binding.reactive_channels() {
        apply_style_atoms(reactive, intent);
    }
    if keep_pad {
        intent.data_mut().padding = pad;
    }
    if let Some(style) = text_style {
        let engine = StyleEngine::new(theme_for(node));
        apply_text_binding(engine, binding, style);
    }
}

/// Seed an image node's intent with its intrinsic pixel size on any axis
/// the author left `Auto`. A pinned `.width()`/`.height()` is already in
/// `intent`, so it is preserved; only the unset axes fall back to the
/// natural extent.
#[allow(clippy::cast_precision_loss)]
fn apply_intrinsic_image_size(intent: LayoutStyle, width: u32, height: u32) -> LayoutStyle {
    let mut intent = intent;
    if intent.data().size.width == Dimension::Auto {
        intent = intent.width(Dimension::Px(width as f32));
    }
    if intent.data().size.height == Dimension::Auto {
        intent = intent.height(Dimension::Px(height as f32));
    }
    intent
}
