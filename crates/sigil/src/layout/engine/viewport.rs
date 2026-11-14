use smallvec::SmallVec;

use crate::Error;
use crate::geometry::{Constraint, Dim, Rect};
use crate::kernel::{Cx, DeltaKind, NodeId};
use crate::layout::{AlignItems, snap};
use crate::widgets::Side;

use super::LayoutEngine;
use super::attribution::contains;
use super::measure::to_available_space;

#[derive(Clone, Copy)]
pub(crate) struct ViewportOverlay;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Popup {
    pub(crate) side: Option<Side>,
    pub(crate) align: AlignItems,
    pub(crate) sibling: bool,
    pub(crate) gap: f32,
}

#[derive(Default)]
struct Found {
    overlays: Vec<NodeId>,
    popups: Vec<(NodeId, Popup)>,
}

impl LayoutEngine {
    pub(super) fn measure_viewport_overlays(
        &mut self,
        cx: &mut Cx,
        root: NodeId,
        available: Constraint,
        solved: &[NodeId],
    ) -> Result<(), Error> {
        let mut found = Found::default();
        collect(cx, root, &mut found);
        for id in found.overlays {
            if !self.overlay_stale(cx, id, solved) {
                continue;
            }
            crate::profile::count(crate::profile::Counter::LayoutOverlay, 1);
            self.drive
                .measure_keep_frame(self.handle(id)?, to_available_space(available))?;
            self.attach_subtree_results(cx, id)?;
        }
        self.popup_offsets.clear();
        let (Dim::Fixed(width), Dim::Fixed(height)) = (available.width, available.height) else {
            return Ok(());
        };
        for (id, popup) in found.popups {
            self.place_popup(cx, id, popup, (width, height));
        }
        Ok(())
    }

    fn overlay_stale(&self, cx: &Cx, id: NodeId, solved: &[NodeId]) -> bool {
        !self.published_result.contains_key(id.key())
            || cx
                .get(id)
                .is_some_and(|node| node.extension::<Popup>().is_some())
            || solved
                .iter()
                .any(|&subroot| subroot != id && contains(cx, subroot, id))
    }

    fn place_popup(&mut self, cx: &mut Cx, id: NodeId, popup: Popup, viewport: (f32, f32)) {
        let Some(rect) = self.result(id).map(|result| result.rect()) else {
            return;
        };
        let anchor = anchor_of(cx, id, popup)
            .and_then(|anchor| self.result(anchor))
            .map(|result| result.rect());
        let (x, y) = target(rect, anchor, popup, viewport);
        let (dx, dy) = (x - rect.origin.x, y - rect.origin.y);
        let moved = dx != 0.0 || dy != 0.0;
        let mut ids = SmallVec::<[NodeId; 32]>::new();
        cx.visit_preorder(id, |node, _| ids.push(node));
        for node in ids {
            let Some(result) = self.result(node) else {
                continue;
            };
            let (ox, oy) = self
                .popup_offsets
                .get(node.key())
                .copied()
                .unwrap_or((0.0, 0.0));
            if moved {
                self.popup_offsets.insert(node.key(), (ox + dx, oy + dy));
            }
            let placed = result.translated(dx, dy);
            let Some(target) = cx.get_mut_untracked(node) else {
                continue;
            };
            if target.extension::<crate::layout::LayoutResult>() == Some(&placed) {
                continue;
            }
            target.attach_untracked(placed);
            cx.delta.record(node, DeltaKind::Layout);
        }
    }
}

fn anchor_of(cx: &Cx, id: NodeId, popup: Popup) -> Option<NodeId> {
    let parent = cx.parent(id)?;
    if !popup.sibling {
        return Some(parent);
    }
    let children = cx.get(parent)?.children();
    let at = children.iter().position(|child| *child == id)?;
    let (before, after) = children.split_at(at);
    let usable = |child: &&NodeId| {
        **child != id
            && cx
                .get(**child)
                .is_some_and(|node| node.extension::<Popup>().is_none())
    };
    before
        .iter()
        .rev()
        .find(usable)
        .or_else(|| after.iter().find(usable))
        .copied()
}

fn target(rect: Rect, anchor: Option<Rect>, popup: Popup, viewport: (f32, f32)) -> (f32, f32) {
    let margins = (snap::gap(popup.gap), snap::row_gap(popup.gap));
    let (Some(anchor), Some(side)) = (anchor, popup.side) else {
        return (
            clamp_axis(rect.origin.x, rect.width, viewport.0, margins.0),
            clamp_axis(rect.origin.y, rect.height, viewport.1, margins.1),
        );
    };
    let horizontal = matches!(side, Side::Left | Side::Right);
    let after = matches!(side, Side::Right | Side::Bottom);
    let main = side_offset(
        axis(anchor, horizontal),
        axis(rect, horizontal).1,
        (after, popup.gap),
        (pick(viewport, horizontal), pick(margins, horizontal)),
    );
    let extent = axis(rect, !horizontal).1;
    let cross = clamp_axis(
        align(axis(anchor, !horizontal), extent, popup.align),
        extent,
        pick(viewport, !horizontal),
        pick(margins, !horizontal),
    );
    if horizontal {
        (main, cross)
    } else {
        (cross, main)
    }
}

const fn axis(rect: Rect, horizontal: bool) -> (f32, f32) {
    if horizontal {
        (rect.origin.x, rect.width)
    } else {
        (rect.origin.y, rect.height)
    }
}

const fn pick(pair: (f32, f32), horizontal: bool) -> f32 {
    if horizontal { pair.0 } else { pair.1 }
}

const fn align((start, length): (f32, f32), size: f32, to: AlignItems) -> f32 {
    match to {
        AlignItems::Center => size.mul_add(-0.5, length.mul_add(0.5, start)),
        AlignItems::End => start + length - size,
        AlignItems::Start | AlignItems::Baseline | AlignItems::Stretch => start,
    }
}

fn side_offset(
    (start, length): (f32, f32),
    size: f32,
    (after, gap): (bool, f32),
    (limit, margin): (f32, f32),
) -> f32 {
    let past = start + length + gap;
    let before = start - gap - size;
    let fits_past = past + size + margin <= limit;
    let fits_before = before >= margin;
    let prefer = if after {
        fits_past || !fits_before
    } else {
        fits_before || !fits_past
    };
    let at = if prefer == after { past } else { before };
    if fits_past || fits_before {
        return clamp_axis(at, size, limit, margin);
    }
    if limit - past >= start { past } else { before }
}

fn clamp_axis(value: f32, size: f32, limit: f32, gap: f32) -> f32 {
    value.min(limit - size - gap).max(gap.min(value).max(0.0))
}

fn collect(cx: &Cx, root: NodeId, found: &mut Found) {
    for id in cx.marked_preorder(&[root]) {
        let Some(node) = cx.get(id) else {
            continue;
        };
        if node.extension::<ViewportOverlay>().is_some() {
            found.overlays.push(id);
        }
        if let Some(popup) = node.extension::<Popup>() {
            found.popups.push((id, *popup));
        }
    }
}
