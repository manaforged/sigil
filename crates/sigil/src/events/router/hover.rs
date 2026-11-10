use crate::events::pointer_dispatch::{PointerTargets, dispatch_synthetic_target, pointer_targets};
use crate::events::{PointerEvent, PointerPhase};
use crate::kernel::Cx;
use crate::layout::LayoutEngine;

use super::EventRouter;

impl EventRouter {
    pub(crate) fn reconcile_hover(&mut self, cx: &Cx, layout: &LayoutEngine) {
        if self.pointer_capture.is_some() {
            return;
        }
        let Some(pointer) = self.last_pointer.clone() else {
            return;
        };
        let seen = (
            cx.mutation_generation(),
            layout.generation(),
            (pointer.position.x.to_bits(), pointer.position.y.to_bits()),
        );
        if self.hover_seen.replace(seen) == Some(seen) {
            return;
        }
        let Some(path) = self.hit_cache.hit_path(cx, layout, pointer.position) else {
            self.clear_hover(cx, &pointer);
            return;
        };
        let next_targets = pointer_targets(cx, layout, &path, self.hit_cache.uses_transforms());
        self.update_hover(cx, &next_targets, &pointer);
    }

    pub(super) fn update_hover(&mut self, cx: &Cx, next: &PointerTargets, source: &PointerEvent) {
        let common = self
            .hover_targets
            .iter()
            .zip(next.iter())
            .take_while(|(a, b)| a.id == b.id)
            .count();
        if common == next.len() && common == self.hover_targets.len() {
            return;
        }
        if let Some(leaving) = self.hover_targets.get(common..) {
            for target in leaving.iter().rev().copied() {
                dispatch_synthetic_target(cx, target, source, PointerPhase::Leave);
            }
        }
        if let Some(entering) = next.get(common..) {
            for target in entering.iter().copied() {
                dispatch_synthetic_target(cx, target, source, PointerPhase::Enter);
            }
        }
        self.hover_targets.clear();
        self.hover_targets.extend(next.iter().copied());
    }

    pub(super) fn clear_hover(&mut self, cx: &Cx, source: &PointerEvent) {
        for target in self.hover_targets.iter().rev().copied() {
            dispatch_synthetic_target(cx, target, source, PointerPhase::Leave);
        }
        self.hover_targets.clear();
    }
}
