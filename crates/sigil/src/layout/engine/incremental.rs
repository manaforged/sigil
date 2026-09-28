use crate::hal::{Dim, Size};
use crate::kernel::{Cx, NodeId, NodeType};
use ahash::AHashSet;
use smallvec::SmallVec;

use crate::layout::style_data::{Dimension, Display};
use crate::layout::{LayoutError, LayoutResult, LayoutStyle};

use super::LayoutEngine;
use super::measure::to_available_space;

impl LayoutEngine {
    #[allow(
        clippy::too_many_arguments,
        reason = "private helper splits the compute path; bundling these would obscure the broadcast/incremental branch"
    )]
    pub(super) fn incremental_sync_and_measure(
        &mut self,
        cx: &mut Cx,
        dirty: &[NodeId],
        root: NodeId,
        available: Size,
        structural: bool,
    ) -> Result<(), LayoutError> {
        let roots = {
            let _dirty = crate::profile::span(crate::profile::Phase::LayoutDirty);
            self.dirty_roots(cx, dirty, root, structural)?
        };
        {
            let _solve = crate::profile::span(crate::profile::Phase::LayoutSolve);
            self.solve_roots(&roots, root, available)?;
        }
        let _attach = crate::profile::span(crate::profile::Phase::LayoutAttach);
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
        if structural {
            self.gc(cx)?;
        }
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
        let resized: Vec<NodeId> = dirty
            .iter()
            .copied()
            .filter(|id| !self.steady.contains(id) && !self.text_kept_size(cx, *id))
            .collect();
        crate::profile::count_add(
            crate::profile::Counter::LayoutDirtyNodes,
            dirty.len() as u64,
        );
        crate::profile::count_add(
            crate::profile::Counter::LayoutResizedNodes,
            resized.len() as u64,
        );
        let roots = self.containment_roots(cx, &resized, root, &repositioned);
        crate::profile::count_add(crate::profile::Counter::LayoutRoots, roots.len() as u64);
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

    fn position_changed(&self, cx: &Cx, id: NodeId) -> bool {
        cx.get(id)
            .and_then(|node| node.get_extension::<crate::layout::ReactivePosition>())
            .is_some_and(|pos| self.positions.get(id.key()).copied() != Some(pos.get()))
    }

    fn moved_only(&self, cx: &Cx, id: NodeId) -> bool {
        let Some(node) = cx.get(id) else {
            return false;
        };
        let Some(pos) = node.get_extension::<crate::layout::ReactivePosition>() else {
            return false;
        };
        self.positions
            .get(id.key())
            .is_some_and(|prior| *prior != pos.get())
            && self.synced_version.get(id.key()).copied() == Some(node.version())
    }

    pub(super) fn translate_moved(
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
            crate::profile::count(crate::profile::Counter::LayoutTranslate);
            self.translate(cx, id)?;
        }
        Ok(moved.len())
    }

    fn translate(&mut self, cx: &mut Cx, id: NodeId) -> Result<(), LayoutError> {
        let node = cx.get(id).ok_or(LayoutError::NodeMissing(id))?;
        let Some(next) = node
            .get_extension::<crate::layout::ReactivePosition>()
            .map(crate::layout::ReactivePosition::get)
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
            let containment = self.find_containment_root(cx, start, root);
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

    fn find_containment_root(&self, cx: &Cx, start: NodeId, root: NodeId) -> NodeId {
        let mut current = start;
        loop {
            if current == root
                || (self.core_to_handle.contains_key(current.key())
                    && Self::has_definite_size(cx, current))
            {
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
                Dimension::Px(_) | Dimension::Percent(_) | Dimension::Fill | Dimension::Text(_)
            )
        };
        definite(style.data().size.width) && definite(style.data().size.height)
    }
}
