use crate::geometry::{Dim, Size};
use crate::kernel::{Cx, NodeId};
use smallvec::SmallVec;

use crate::layout::forest::{AvailableSpace, SpaceConstraint};
use crate::layout::{LayoutError, LayoutResult};

use super::{LayoutEngine, note_full};

#[derive(Clone, Copy)]
pub(super) enum Pass<'a> {
    Remeasure,
    Full {
        mirrored: bool,
        dirty: &'a [NodeId],
    },
    Incremental {
        structural: bool,
        dirty: &'a [NodeId],
    },
}

impl<'a> Pass<'a> {
    pub(super) const fn changed(
        dirty: &'a [NodeId],
        majority: bool,
        mirrored: bool,
        structural: bool,
    ) -> Self {
        if majority || dirty.is_empty() {
            Self::Full { mirrored, dirty }
        } else {
            Self::Incremental { structural, dirty }
        }
    }
}

impl LayoutEngine {
    pub(super) fn solve_pass(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
        pass: Pass<'_>,
    ) -> Result<SmallVec<[NodeId; 8]>, LayoutError> {
        match pass {
            Pass::Remeasure => {
                crate::profile::count(crate::profile::Counter::LayoutResize);
                self.remeasure_and_publish(cx, root, available)?;
            }
            Pass::Full { mirrored, dirty } => {
                note_full(cx, root, mirrored, dirty);
                self.full_sync_and_measure(cx, root, available)?;
            }
            Pass::Incremental { structural, dirty } => {
                crate::profile::count(crate::profile::Counter::LayoutIncr);
                let roots =
                    self.incremental_sync_and_measure(cx, dirty, root, available, structural)?;
                return Ok(roots.into_iter().collect());
            }
        }
        Ok(smallvec::smallvec![root])
    }

    fn compute_mirror(&mut self, root: NodeId, available: Size) -> Result<(), LayoutError> {
        let _solve = crate::profile::span(crate::profile::Phase::LayoutSolve);
        self.drive.clear_intrinsic();
        self.drive
            .measure_with_constraints(self.handle(root)?, to_available_space(available))
    }

    pub(super) fn remeasure_and_publish(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        self.compute_mirror(root, available)?;
        let _attach = crate::profile::span(crate::profile::Phase::LayoutAttach);
        self.attach_subtree_results(cx, root)
    }

    pub(super) fn full_sync_and_measure(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Size,
    ) -> Result<(), LayoutError> {
        let previous_ids: SmallVec<[NodeId; 32]> =
            self.core_to_handle.keys().map(NodeId::from_key).collect();
        {
            let _sync = crate::profile::span(crate::profile::Phase::LayoutSync);
            self.sync(cx, root)?;
        }
        self.compute_mirror(root, available)?;
        self.remove_stale_extensions(cx, previous_ids);
        let _attach = crate::profile::span(crate::profile::Phase::LayoutAttach);
        self.attach_subtree_results(cx, root)
    }

    pub(super) fn remeasure_roots(
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

    pub(super) fn attach_subtree_results(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
    ) -> Result<(), LayoutError> {
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
            let node = cx
                .get_mut_untracked(id)
                .ok_or(LayoutError::NodeMissing(id))?;
            let _ = node.attach_untracked(result);
            let rect = result.rect();
            if let Some(size) = node.get_extension::<crate::layout::SizeReactive>() {
                self.republished |= size.publish((rect.width, rect.height));
            }
            if let Some(bounds) = node.get_extension::<crate::layout::BoundsReactive>() {
                self.republished |=
                    bounds.publish((rect.origin.x, rect.origin.y, rect.width, rect.height));
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
            if let Some(node) = cx.get_mut_untracked(id)
                && node.get_extension::<LayoutResult>().is_some()
            {
                let _ = node.remove_extension::<LayoutResult>();
            }
        }
    }
}

pub(super) const fn to_available_space(size: Size) -> AvailableSpace {
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
