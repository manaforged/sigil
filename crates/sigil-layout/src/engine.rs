//! Layout engine and `sigil-core` synchronization.

use ahash::AHashMap;
use sigil_core::{Cx, NodeId, NodeType, TextContent};
use sigil_hal::{Dim, Size};
use smallvec::SmallVec;
use taffy::prelude::{AvailableSpace, NodeId as TaffyId, Size as TaffySize, TaffyTree};

use crate::style_adapter::apply_style_atoms;
use crate::{LayoutError, LayoutResult, LayoutStyle, TextMeasure};

/// Taffy-backed layout engine.
pub struct LayoutEngine {
    taffy: TaffyTree<LayoutContext>,
    core_to_taffy: AHashMap<NodeId, TaffyId>,
    taffy_to_core: AHashMap<TaffyId, NodeId>,
    computed: AHashMap<NodeId, LayoutResult>,
    measurer: Box<dyn TextMeasure>,
    generation: u64,
}

#[derive(Clone, Debug)]
struct LayoutContext {
    text_content: Option<TextContent>,
}

impl LayoutEngine {
    /// Create a layout engine with the given text measurer.
    #[must_use]
    pub fn with_text_measure(measurer: Box<dyn TextMeasure>) -> Self {
        Self {
            taffy: TaffyTree::new(),
            core_to_taffy: AHashMap::new(),
            taffy_to_core: AHashMap::new(),
            computed: AHashMap::new(),
            measurer,
            generation: 0,
        }
    }

    /// Return the number of nodes currently mirrored into Taffy.
    #[must_use]
    pub fn mirrored_node_count(&self) -> usize {
        self.core_to_taffy.len()
    }

    /// Monotonic counter bumped on every successful `compute` call.
    ///
    /// Downstream caches keyed on layout geometry should invalidate
    /// when this value changes, paired with
    /// [`Cx::mutation_generation`](sigil_core::Cx::mutation_generation)
    /// for tree-shape invalidation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Return the most recently computed result for a node.
    #[must_use]
    pub fn result(&self, id: NodeId) -> Option<LayoutResult> {
        self.computed.get(&id).copied()
    }

    /// Mirror a `sigil-core` subtree into the private Taffy tree.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::NodeMissing`] when `root` is stale or a
    /// child mapping is absent, and returns [`LayoutError::TaffyInvalid`]
    /// when Taffy rejects a mirror update.
    pub fn sync(&mut self, cx: &Cx, root: NodeId) -> Result<(), LayoutError> {
        self.sync_subtree(cx, root).map(|_taffy_ids| ())
    }

    /// Compute layout for a `sigil-core` subtree and attach results back to nodes.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::NodeMissing`] when `root` or a computed
    /// node is stale, and returns [`LayoutError::TaffyInvalid`] when
    /// Taffy rejects sync or compute.
    pub fn compute(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        self.measurer.begin_frame();
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
            // Fragments have no Taffy node and no layout result — skip them.
            if let Some(result) = self.result(id) {
                let node = cx.get_mut(id).ok_or(LayoutError::NodeMissing(id))?;
                let _ = node.attach(result);
            }
        }

        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    /// Remove mirrored Taffy nodes for `sigil-core` nodes no longer present.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::TaffyInvalid`] when Taffy rejects a
    /// removal while collecting stale nodes.
    pub fn gc(&mut self, cx: &Cx) -> Result<(), LayoutError> {
        let stale = self
            .core_to_taffy
            .keys()
            .copied()
            .filter(|id| cx.get(*id).is_none())
            .collect::<Vec<_>>();

        for id in stale {
            if self.core_to_taffy.contains_key(&id) {
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
        self.upsert_with_text(id, intent, None)
    }

    pub(crate) fn set_children_mirror(
        &mut self,
        parent: NodeId,
        children: &[NodeId],
    ) -> Result<(), LayoutError> {
        let parent_taffy = self.taffy_id(parent)?;
        let mut taffy_children = SmallVec::<[TaffyId; 8]>::new();
        for child in children {
            taffy_children.push(self.taffy_id(*child)?);
        }
        self.taffy
            .set_children(parent_taffy, &taffy_children)
            .map_err(LayoutError::from)
    }

    pub(crate) fn remove_mirror(&mut self, id: NodeId) -> Result<(), LayoutError> {
        let taffy_id = self
            .core_to_taffy
            .get(&id)
            .copied()
            .ok_or(LayoutError::NodeMissing(id))?;

        let mut subtree = Vec::new();
        self.collect_taffy_subtree(taffy_id, &mut subtree)?;
        for taffy_node in subtree.into_iter().rev() {
            if let Some(core_id) = self.taffy_to_core.remove(&taffy_node) {
                let _ = self.core_to_taffy.remove(&core_id);
                let _ = self.computed.remove(&core_id);
            }
            self.taffy.remove(taffy_node).map_err(LayoutError::from)?;
        }

        Ok(())
    }

    pub(crate) fn compute_mirror(
        &mut self,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        let root_taffy = self.taffy_id(root)?;
        let available = to_taffy_size(available);
        let measurer = &self.measurer;

        self.taffy
            .compute_layout_with_measure(
                root_taffy,
                available,
                |known, space, _node_id, context, _style| {
                    measure_text(measurer.as_ref(), known, space, context)
                },
            )
            .map_err(LayoutError::from)?;

        self.refresh_results(root_taffy)
    }

    /// Sync one node into Taffy and return the Taffy IDs the *caller* should
    /// register as its children.
    ///
    /// For `Container` and `Text` nodes this is `[taffy_id_of_self]`.
    /// For `Fragment` nodes there is no Taffy node — the fragment's own
    /// children are returned directly so they appear as siblings of the
    /// fragment inside its parent's layout box.
    fn sync_subtree(&mut self, cx: &Cx, id: NodeId) -> Result<SmallVec<[TaffyId; 4]>, LayoutError> {
        let node = cx.get(id).ok_or(LayoutError::NodeMissing(id))?;
        let children = node.children.iter().copied().collect::<SmallVec<[_; 4]>>();

        // Fragments flatten into their parent: no Taffy node of their own.
        if matches!(node.kind, NodeType::Fragment) {
            let mut out = SmallVec::<[TaffyId; 4]>::new();
            for child in &children {
                out.extend(self.sync_subtree(cx, *child)?);
            }
            return Ok(out);
        }

        let mut intent = node
            .get_extension::<LayoutStyle>()
            .cloned()
            .unwrap_or_default();
        if let Some(binding) = node.get_extension::<sigil_style::StyleBinding>() {
            apply_style_atoms(binding.static_slice(), &mut intent);
            apply_style_atoms(binding.dynamic_slice(), &mut intent);
            apply_style_atoms(binding.reactive_slice(), &mut intent);
        }
        let text_content = match &node.kind {
            NodeType::Text { content } => Some(content.clone()),
            NodeType::Container | NodeType::Fragment => None,
        };

        self.upsert_with_text(id, &intent, text_content)?;

        let mut taffy_children = SmallVec::<[TaffyId; 8]>::new();
        for child in &children {
            taffy_children.extend(self.sync_subtree(cx, *child)?);
        }
        let parent_taffy = self.taffy_id(id)?;
        self.taffy
            .set_children(parent_taffy, &taffy_children)
            .map_err(LayoutError::from)?;

        let mut out = SmallVec::<[TaffyId; 4]>::new();
        out.push(parent_taffy);
        Ok(out)
    }

    fn upsert_with_text(
        &mut self,
        id: NodeId,
        intent: &LayoutStyle,
        text_content: Option<TextContent>,
    ) -> Result<(), LayoutError> {
        let style = intent.as_taffy().clone();
        let context = LayoutContext { text_content };

        if let Some(taffy_id) = self.core_to_taffy.get(&id).copied() {
            self.taffy
                .set_style(taffy_id, style)
                .map_err(LayoutError::from)?;
            self.taffy
                .set_node_context(taffy_id, Some(context))
                .map_err(LayoutError::from)?;
            return Ok(());
        }

        let taffy_id = self
            .taffy
            .new_leaf_with_context(style, context)
            .map_err(LayoutError::from)?;
        self.core_to_taffy.insert(id, taffy_id);
        self.taffy_to_core.insert(taffy_id, id);
        Ok(())
    }

    fn refresh_results(&mut self, root: TaffyId) -> Result<(), LayoutError> {
        let mut stack = vec![root];
        let mut computed = AHashMap::new();
        while let Some(taffy_id) = stack.pop() {
            let core_id =
                self.taffy_to_core.get(&taffy_id).copied().ok_or_else(|| {
                    LayoutError::TaffyInvalid("missing inverse node map".to_owned())
                })?;
            let layout = self.taffy.layout(taffy_id).map_err(LayoutError::from)?;
            computed.insert(core_id, LayoutResult::from_taffy(layout));
            let children = self.taffy.children(taffy_id).map_err(LayoutError::from)?;
            stack.extend(children);
        }
        self.computed = computed;
        Ok(())
    }

    fn collect_taffy_subtree(
        &self,
        root: TaffyId,
        out: &mut Vec<TaffyId>,
    ) -> Result<(), LayoutError> {
        out.push(root);
        let children = self.taffy.children(root).map_err(LayoutError::from)?;
        for child in children {
            self.collect_taffy_subtree(child, out)?;
        }
        Ok(())
    }

    fn taffy_id(&self, id: NodeId) -> Result<TaffyId, LayoutError> {
        self.core_to_taffy
            .get(&id)
            .copied()
            .ok_or(LayoutError::NodeMissing(id))
    }
}

fn measure_text(
    measurer: &dyn TextMeasure,
    known: TaffySize<Option<f32>>,
    space: TaffySize<AvailableSpace>,
    context: Option<&mut LayoutContext>,
) -> TaffySize<f32> {
    let text_size = context
        .and_then(|context| {
            context
                .text_content
                .as_ref()
                .map(|source| measurer.measure(source, space))
        })
        .unwrap_or_default();

    TaffySize {
        width: known.width.unwrap_or(text_size.width),
        height: known.height.unwrap_or(text_size.height),
    }
}

const fn to_taffy_size(size: Size) -> TaffySize<AvailableSpace> {
    TaffySize {
        width: to_available_space(size.width),
        height: to_available_space(size.height),
    }
}

const fn to_available_space(dim: Dim) -> AvailableSpace {
    match dim {
        Dim::Fixed(value) => AvailableSpace::Definite(value),
        Dim::MinContent => AvailableSpace::MinContent,
        Dim::MaxContent => AvailableSpace::MaxContent,
    }
}
