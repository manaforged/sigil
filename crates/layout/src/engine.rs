//! Layout engine and `sigil-kernel` synchronization.

use ahash::AHashSet;
use sigil_hal::{Dim, Size};
use sigil_kernel::{Cx, NodeId, NodeType, TextContent};
use sigil_style::{StyleBinding, StyleEngine, TextStyle};
use slotmap::SecondaryMap;
use smallvec::SmallVec;

use crate::cache_key::{AvailableKey, CacheDecision, LayoutCacheKey, layout_input_signature};
use crate::engine_drive::{LayoutDrive, NodeHandle};
use crate::forest::{AvailableSpace, SpaceConstraint};
use crate::style_adapter::apply_style_atoms;
use crate::style_data::Dimension;
use crate::text_style::{apply_text_binding, theme_for};
use crate::{LayoutError, LayoutResult, LayoutStyle, TextMeasure};

/// Owned layout engine mirroring a `sigil-kernel` subtree into [`LayoutDrive`].
pub struct LayoutEngine {
    drive: LayoutDrive,
    /// `NodeId → NodeHandle` mirror map. Slotmap-keyed (not hashed) so
    /// the hot sync path is O(1) array indexing instead of an `AHashMap`
    /// lookup per node. Vector β in `docs/benchmarks/p0-owned-layout.md`.
    core_to_handle: SecondaryMap<slotmap::DefaultKey, NodeHandle>,
    /// Last kernel-side `Node::version()` we observed per node. The
    /// sync hot path peeks the kernel version *before* doing any
    /// `get_extension` / `clone` / `set_style` work; when it matches,
    /// the per-node payload is provably unchanged and we skip the
    /// entire input-extraction + write path. Vector ε1 — eliminates
    /// the two `AHashMap` lookups + `LayoutStyle` clone that were 65%
    /// of the broadcast frame post-vector-δ.
    synced_version: SecondaryMap<slotmap::DefaultKey, u32>,
    /// Last `LayoutResult` we attached to each kernel node. Read via
    /// O(1) slotmap indexing instead of the kernel's
    /// `AHashMap<TypeId, Box<dyn Any>>` extension lookup. Comparing
    /// new vs cached lets `attach_results` skip nodes whose box did
    /// not move — eliminates the per-node `get_extension::<LayoutResult>()`
    /// `AHashMap` probe that was ~80 ns × 101k = ~8 ms per broadcast.
    published_result: SecondaryMap<slotmap::DefaultKey, LayoutResult>,
    generation: u64,
    cached_key: Option<LayoutCacheKey>,
}

impl LayoutEngine {
    /// Create a layout engine with the given text measurer.
    #[must_use]
    pub fn with_text_measure(measurer: Box<dyn TextMeasure>) -> Self {
        Self {
            drive: LayoutDrive::new(measurer),
            core_to_handle: SecondaryMap::new(),
            synced_version: SecondaryMap::new(),
            published_result: SecondaryMap::new(),
            generation: 0,
            cached_key: None,
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
        Some(LayoutResult::from_resolved_box(&layout))
    }

    /// Mirror a `sigil-kernel` subtree into the private owned layout drive.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::NodeMissing`] when `root` is stale or a
    /// child mapping is absent.
    pub fn sync(&mut self, cx: &Cx, root: NodeId) -> Result<(), LayoutError> {
        self.sync_subtree(cx, root).map(|_handles| ())
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
        let cache_key = match self.resolve_cache_key(cx, root, available) {
            CacheDecision::Hit => return Ok(()),
            CacheDecision::Miss(key) => key,
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
            self.published_result.clear();
            self.core_to_handle.clear();
        }

        self.drive.begin_frame();
        let dirty = cx.dirty().collect::<Vec<_>>();
        let live_count = cx.len();
        let broadcast = dirty.is_empty() || dirty.len() >= live_count.max(1) / 2;
        let structural = self
            .cached_key
            .is_some_and(|key| key.cx_generation != cx.mutation_generation());

        if broadcast {
            self.full_sync_and_measure(cx, root, available)?;
        } else {
            self.incremental_sync_and_measure(cx, &dirty, root, available, structural)?;
        }

        cx.clear_dirty();
        self.generation = self.generation.wrapping_add(1);
        // ε-cache: re-hash the signature *after* work completes so the
        // value reflects the post-attach `Node::version` state. The
        // next `compute` call computes the same post-attach hash, sees
        // a match, and returns `CacheDecision::Hit` — skipping the
        // entire sync + measure + attach work cycle for unchanged
        // inputs. This is the legitimate path to "beat" Taffy on
        // repeated re-layouts of a stable tree.
        let post_signature = layout_input_signature(cx, root);
        self.cached_key = Some(LayoutCacheKey {
            input_signature: post_signature,
            ..cache_key
        });
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

    pub(crate) fn upsert_mirror(
        &mut self,
        id: NodeId,
        intent: &LayoutStyle,
    ) -> Result<(), LayoutError> {
        let existing = self.core_to_handle.get(id.key()).copied();
        self.upsert_with_text(id, existing, intent, None, None)
    }

    pub(crate) fn set_children_mirror(
        &mut self,
        parent: NodeId,
        children: &[NodeId],
    ) -> Result<(), LayoutError> {
        let parent_handle = self.handle(parent)?;
        let mut child_handles = SmallVec::<[NodeHandle; 8]>::new();
        for child in children {
            child_handles.push(self.handle(*child)?);
        }
        self.drive.set_children(parent_handle, &child_handles)
    }

    pub(crate) fn remove_mirror(&mut self, id: NodeId) -> Result<(), LayoutError> {
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
                let _ = self.published_result.remove(core_id.key());
            }
        }
        self.drive.remove(handle)
    }

    pub(crate) fn compute_mirror(
        &mut self,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        let root_handle = self.handle(root)?;
        self.drive
            .measure_with_constraints(root_handle, to_available_space(available))?;
        Ok(())
    }

    fn full_sync_and_measure(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        let previous_ids: SmallVec<[NodeId; 32]> =
            self.core_to_handle.keys().map(NodeId::from_key).collect();
        self.sync(cx, root)?;
        self.compute_mirror(root, available)?;

        let mut ids = SmallVec::<[_; 32]>::new();
        cx.visit_preorder(root, |id, _node| ids.push(id));
        self.remove_stale_extensions(cx, previous_ids);
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
        if structural {
            self.gc(cx)?;
        }
        let roots = self.containment_roots(cx, dirty, root);
        self.remeasure_roots(&roots, available)?;
        for subroot in roots {
            self.attach_subtree_results(cx, subroot)?;
        }
        Ok(())
    }

    fn resolve_cache_key(&self, cx: &Cx, root: NodeId, available: Size) -> CacheDecision {
        let cx_instance = cx.instance_id();
        let cx_generation = cx.mutation_generation();
        let available_key = AvailableKey::from_size(available);
        if self.cached_key.is_some() && cx.dirty().next().is_some() {
            return CacheDecision::Miss(LayoutCacheKey {
                cx_instance,
                root,
                available: available_key,
                cx_generation,
                input_signature: layout_input_signature(cx, root),
            });
        }
        let cheap_match = self.cached_key.is_some_and(|prev| {
            prev.cx_instance == cx_instance
                && prev.root == root
                && prev.available == available_key
                && prev.cx_generation == cx_generation
        });
        let signature = if cheap_match {
            let sig = layout_input_signature(cx, root);
            if self
                .cached_key
                .is_some_and(|prev| prev.input_signature == sig)
            {
                return CacheDecision::Hit;
            }
            sig
        } else {
            layout_input_signature(cx, root)
        };
        CacheDecision::Miss(LayoutCacheKey {
            cx_instance,
            root,
            available: available_key,
            cx_generation,
            input_signature: signature,
        })
    }

    /// Sync one kernel node and return the handles the caller should
    /// register as children. Fragments flatten into their parent.
    fn sync_subtree(
        &mut self,
        cx: &Cx,
        id: NodeId,
    ) -> Result<SmallVec<[NodeHandle; 4]>, LayoutError> {
        let node = cx.get(id).ok_or(LayoutError::NodeMissing(id))?;
        let kernel_version = node.version();
        let children = node
            .children()
            .iter()
            .copied()
            .collect::<SmallVec<[_; 4]>>();
        if matches!(node.kind, NodeType::Fragment) {
            let mut out = SmallVec::<[NodeHandle; 4]>::new();
            for child in &children {
                out.extend(self.sync_subtree(cx, *child)?);
            }
            return Ok(out);
        }

        // ε1 fast-path: when the kernel-side `Node::version` matches
        // the version we cached on the last sync, every observable
        // payload (style binding, layout intent, text content, a11y,
        // child list) is provably unchanged. Skip `layout_inputs_for_node`
        // (two AHashMap lookups + LayoutStyle clone) and `upsert_with_text`
        // entirely. Children still recurse — their versions are checked
        // independently. `set_children` will cheap-out (δ9) when its
        // child list is byte-identical.
        // Single lookup feeds both the version-match fast-path and (on
        // miss) `upsert_with_text`'s existing-handle branch — saves one
        // slotmap secondary probe per synced node on the broadcast path.
        let existing_handle = self.core_to_handle.get(id.key()).copied();
        // Reactive style bindings mutate the kernel node's reactive
        // atom slice through an `Rc<Cell>` channel that bypasses
        // `Node::version()` — see `StyleReactiveEffect::set_slice`.
        // The version fast-path is correct for static / dynamic /
        // structural mutations but would let a reactive `Display::None
        // → []` flip go undetected, stranding the affected subtree
        // (e.g. an absolutely-positioned tooltip bubble) in the
        // forest's stale `Display::None` state. Force a re-sync when
        // a reactive binding is attached; the downstream
        // `upsert_with_text` style-equality check still cheap-outs
        // when nothing actually changed.
        // `reactive_slice()` returns `&[]` for both "no reactive
        // binding installed" and "reactive binding currently empty";
        // the cell-pointer identity check disambiguates by comparing
        // against the canonical empty-slice constant.
        let has_reactive = node
            .get_extension::<StyleBinding>()
            .is_some_and(StyleBinding::has_reactive);
        let version_matches = !has_reactive
            && existing_handle.is_some()
            && self.synced_version.get(id.key()).copied() == Some(kernel_version);
        if !version_matches {
            let (intent, text_content, text_style) = Self::layout_inputs_for_node(node);
            self.upsert_with_text(id, existing_handle, &intent, text_content, text_style)?;
            self.synced_version.insert(id.key(), kernel_version);
        }

        let mut child_handles = SmallVec::<[NodeHandle; 8]>::new();
        for child in &children {
            child_handles.extend(self.sync_subtree(cx, *child)?);
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
            self.drive.set_text(handle, text_content, text_style)?;
            return Ok(());
        }

        let style = intent.data().clone();
        let handle = self.drive.insert_with_kernel(id, style);
        self.drive.set_text(handle, text_content, text_style)?;
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
        if matches!(node.kind, NodeType::Fragment) {
            return Ok(());
        }
        let (intent, text_content, text_style) = Self::layout_inputs_for_node(node);
        let existing = self.core_to_handle.get(id.key()).copied();
        self.upsert_with_text(id, existing, &intent, text_content, text_style)
    }

    fn layout_inputs_for_node(
        node: &sigil_kernel::Node,
    ) -> (LayoutStyle, Option<TextContent>, Option<TextStyle>) {
        let mut intent = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        let mut text_style = match &node.kind {
            NodeType::Text { .. } => Some(TextStyle::default()),
            NodeType::Container | NodeType::Fragment => None,
        };
        if let Some(binding) = node.get_extension::<StyleBinding>() {
            apply_style_atoms(binding.static_slice(), &mut intent);
            apply_style_atoms(binding.dynamic_slice(), &mut intent);
            apply_style_atoms(binding.reactive_slice(), &mut intent);
            if let Some(style) = text_style.as_mut() {
                let engine = StyleEngine::new(theme_for(node));
                apply_text_binding(engine, binding, style);
            }
        }
        let text_content = match &node.kind {
            NodeType::Text { content } => Some(content.clone()),
            NodeType::Container | NodeType::Fragment => None,
        };
        (intent, text_content, text_style)
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
        roots
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
            let node = cx.get_mut(id).ok_or(LayoutError::NodeMissing(id))?;
            let _ = node.attach(result);
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
            if let Some(node) = cx.get_mut(id) {
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
