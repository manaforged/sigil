//! Layout engine and `sigil-kernel` synchronization.

use ahash::AHashMap;
use sigil_hal::{Dim, Size};
use sigil_kernel::{Cx, NodeId, NodeType, TextContent};
use sigil_style::{StyleBinding, StyleEngine, TextStyle};
use smallvec::SmallVec;

use crate::cache_key::{AvailableKey, CacheDecision, LayoutCacheKey, layout_input_signature};
use crate::engine_drive::{LayoutDrive, NodeHandle};
use crate::forest::{AvailableSpace, SpaceConstraint};
use crate::style_adapter::apply_style_atoms;
use crate::text_style::{apply_text_binding, theme_for};
use crate::{LayoutError, LayoutResult, LayoutStyle, TextMeasure};

/// Owned layout engine mirroring a `sigil-kernel` subtree into [`LayoutDrive`].
pub struct LayoutEngine {
    drive: LayoutDrive,
    core_to_handle: AHashMap<NodeId, NodeHandle>,
    handle_to_core: AHashMap<NodeHandle, NodeId>,
    computed: AHashMap<NodeId, LayoutResult>,
    generation: u64,
    cached_key: Option<LayoutCacheKey>,
}

impl LayoutEngine {
    /// Create a layout engine with the given text measurer.
    #[must_use]
    pub fn with_text_measure(measurer: Box<dyn TextMeasure>) -> Self {
        Self {
            drive: LayoutDrive::new(measurer),
            core_to_handle: AHashMap::new(),
            handle_to_core: AHashMap::new(),
            computed: AHashMap::new(),
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

    /// Return the most recently computed result for a node.
    #[must_use]
    pub fn result(&self, id: NodeId) -> Option<LayoutResult> {
        self.computed.get(&id).copied()
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

        self.drive.begin_frame();
        let previous_ids = self.computed.keys().copied().collect::<SmallVec<[_; 32]>>();
        self.sync(cx, root)?;
        self.compute_mirror(root, available)?;

        let mut ids = SmallVec::<[_; 32]>::new();
        cx.visit_preorder(root, |id, _node| ids.push(id));

        for id in previous_ids {
            if self.computed.contains_key(&id) {
                continue;
            }
            if let Some(node) = cx.get_mut(id) {
                let _ = node.remove_extension::<LayoutResult>();
            }
        }

        for id in ids {
            if let Some(result) = self.result(id) {
                let node = cx.get_mut(id).ok_or(LayoutError::NodeMissing(id))?;
                let _ = node.attach(result);
            }
        }

        self.generation = self.generation.wrapping_add(1);
        self.cached_key = Some(cache_key);
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
            .copied()
            .filter(|id| cx.get(*id).is_none())
            .collect::<Vec<_>>();

        for id in stale {
            if self.core_to_handle.contains_key(&id) {
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
        self.upsert_with_text(id, intent, None, None)
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
            .get(&id)
            .copied()
            .ok_or(LayoutError::NodeMissing(id))?;
        let subtree = self.drive.subtree_handles(handle)?;

        for handle in &subtree {
            if let Some(core_id) = self.handle_to_core.remove(handle) {
                let _ = self.core_to_handle.remove(&core_id);
                let _ = self.computed.remove(&core_id);
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
        self.refresh_results(root_handle)
    }

    fn resolve_cache_key(&self, cx: &Cx, root: NodeId, available: Size) -> CacheDecision {
        let cx_instance = cx.instance_id();
        let cx_generation = cx.mutation_generation();
        let available_key = AvailableKey::from_size(available);
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

        self.upsert_with_text(id, &intent, text_content, text_style)?;

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

    fn upsert_with_text(
        &mut self,
        id: NodeId,
        intent: &LayoutStyle,
        text_content: Option<TextContent>,
        text_style: Option<TextStyle>,
    ) -> Result<(), LayoutError> {
        let style = intent.data().clone();
        if let Some(handle) = self.core_to_handle.get(&id).copied() {
            self.drive.set_style(handle, style)?;
            self.drive.set_text(handle, text_content, text_style)?;
            return Ok(());
        }

        let handle = self.drive.insert(style);
        self.drive.set_text(handle, text_content, text_style)?;
        self.core_to_handle.insert(id, handle);
        self.handle_to_core.insert(handle, id);
        Ok(())
    }

    fn refresh_results(&mut self, root: NodeHandle) -> Result<(), LayoutError> {
        let mut computed = AHashMap::new();
        for handle in self.drive.subtree_handles(root)? {
            let core_id =
                self.handle_to_core.get(&handle).copied().ok_or_else(|| {
                    LayoutError::LayoutInvalid("missing inverse node map".to_owned())
                })?;
            if let Some(layout) = self.drive.computed_box(handle) {
                computed.insert(core_id, LayoutResult::from_resolved_box(&layout));
            }
        }
        self.computed = computed;
        Ok(())
    }

    fn handle(&self, id: NodeId) -> Result<NodeHandle, LayoutError> {
        self.core_to_handle
            .get(&id)
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
