use smallvec::SmallVec;

use crate::hal::{Dim, Rect, Size};
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutError;
use crate::layout::snap;
use crate::style::StyleBinding;

use super::LayoutEngine;
use super::measure::to_available_space;

#[derive(Clone, Copy)]
pub(crate) struct ViewportOverlay;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PopupSide {
    Keep,
    Below,
    Above,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Popup {
    pub(crate) side: PopupSide,
    pub(crate) center: bool,
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
        available: Size,
    ) -> Result<(), LayoutError> {
        let mut found = Found::default();
        collect(cx, root, &mut found);
        for id in found.overlays {
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

    fn place_popup(&mut self, cx: &mut Cx, id: NodeId, popup: Popup, viewport: (f32, f32)) {
        let Some(rect) = self.result(id).map(|result| result.rect()) else {
            return;
        };
        let anchor = anchor_of(cx, id, popup)
            .and_then(|anchor| self.result(anchor))
            .map(|result| result.rect());
        let (x, y) = target(rect, anchor, popup, viewport);
        let (dx, dy) = (x - rect.origin.x, y - rect.origin.y);
        if dx == 0.0 && dy == 0.0 {
            return;
        }
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
            self.popup_offsets.insert(node.key(), (ox + dx, oy + dy));
            if let Some(target) = cx.get_mut_untracked(node) {
                target.attach_untracked(result.translated(dx, dy));
            }
        }
    }
}

fn anchor_of(cx: &Cx, id: NodeId, popup: Popup) -> Option<NodeId> {
    let parent = cx.parent(id)?;
    if !popup.sibling {
        return Some(parent);
    }
    cx.get(parent)?.children().iter().copied().find(|child| {
        *child != id
            && cx
                .get(*child)
                .is_some_and(|node| node.get_extension::<Popup>().is_none())
    })
}

fn target(rect: Rect, anchor: Option<Rect>, popup: Popup, viewport: (f32, f32)) -> (f32, f32) {
    let (width, height) = (rect.width, rect.height);
    let x = anchor
        .filter(|_| popup.side != PopupSide::Keep)
        .map_or(rect.origin.x, |anchor| align_x(anchor, width, popup.center));
    let x = clamp_axis(x, width, viewport.0, snap::gap(popup.gap));
    let margin = snap::row_gap(popup.gap);
    let y = match anchor {
        Some(anchor) if popup.side != PopupSide::Keep => {
            side_y(anchor, height, popup, (viewport.1, margin))
        }
        _ => clamp_axis(rect.origin.y, height, viewport.1, margin),
    };
    (x, y)
}

#[allow(
    clippy::missing_const_for_fn,
    reason = "const f32::mul_add needs Rust 1.94; the MSRV is 1.90"
)]
fn align_x(anchor: Rect, width: f32, center: bool) -> f32 {
    if center {
        width.mul_add(-0.5, anchor.width.mul_add(0.5, anchor.origin.x))
    } else {
        anchor.origin.x
    }
}

fn side_y(anchor: Rect, height: f32, popup: Popup, (limit, margin): (f32, f32)) -> f32 {
    let below = anchor.origin.y + anchor.height + popup.gap;
    let above = anchor.origin.y - popup.gap - height;
    let fits_below = below + height + margin <= limit;
    let fits_above = above >= margin;
    let y = match popup.side {
        PopupSide::Above if fits_above || !fits_below => above,
        PopupSide::Below if fits_below || !fits_above => below,
        PopupSide::Above => below,
        PopupSide::Below | PopupSide::Keep => above,
    };
    if fits_below || fits_above {
        return clamp_axis(y, height, limit, margin);
    }
    if limit - below >= anchor.origin.y {
        below
    } else {
        above
    }
}

fn clamp_axis(value: f32, size: f32, limit: f32, gap: f32) -> f32 {
    value.min(limit - size - gap).max(gap.min(value).max(0.0))
}

fn collect(cx: &Cx, id: NodeId, found: &mut Found) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if node
        .get_extension::<StyleBinding>()
        .is_some_and(StyleBinding::hides)
    {
        return;
    }
    if node.get_extension::<ViewportOverlay>().is_some() {
        found.overlays.push(id);
    }
    if let Some(popup) = node.get_extension::<Popup>() {
        found.popups.push((id, *popup));
    }
    for child in node.children() {
        collect(cx, *child, found);
    }
}
