use smallvec::SmallVec;

use crate::geometry::{Point, Rect};
use crate::kernel::{Cx, DeltaKind, NodeId};
use crate::layout::resolve::{NodeEdges, resolve_lp_auto};
use crate::layout::shift::{Shift, ShiftSlot};
use crate::layout::snap;
use crate::layout::style_data::{Edges, Position, StyleData};

use super::LayoutEngine;

pub(super) struct Stuck {
    slot: ShiftSlot,
    by: (f32, f32),
}

impl LayoutEngine {
    pub(super) fn track_sticky(&mut self, id: NodeId) {
        let sticky = self
            .style_of(id)
            .is_some_and(|style| style.position == Position::Sticky);
        if sticky == self.stuck.contains_key(id.key()) {
            return;
        }
        if sticky {
            let stuck = Stuck {
                slot: ShiftSlot::new(),
                by: (0.0, 0.0),
            };
            let _ = self.stuck.insert(id.key(), stuck);
        } else {
            let _ = self.stuck.remove(id.key());
        }
        self.scopes.clear();
    }

    pub(super) fn sticky_scope(&self, id: NodeId, under: Shift) -> Option<Shift> {
        self.stuck.get(id.key()).map(|stuck| stuck.slot.nest(under))
    }

    pub(super) fn stick(&mut self, cx: &Cx, id: NodeId) -> bool {
        if !self.stuck.contains_key(id.key()) {
            return false;
        }
        let by = self.sticky_offset(cx, id).unwrap_or((0.0, 0.0));
        let Some(stuck) = self.stuck.get_mut(id.key()) else {
            return false;
        };
        if stuck.by == by {
            return false;
        }
        stuck.by = by;
        stuck.slot.move_to(by);
        true
    }

    pub(super) fn restick(&mut self, cx: &Cx) {
        if self.stuck.is_empty() {
            return;
        }
        let mut ids: SmallVec<[(usize, NodeId); 8]> = self
            .stuck
            .keys()
            .map(NodeId::from_key)
            .map(|id| {
                (
                    std::iter::successors(cx.parent(id), |&at| cx.parent(at)).count(),
                    id,
                )
            })
            .collect();
        ids.sort_unstable_by_key(|(depth, _)| *depth);
        for (_, id) in ids {
            let _ = self.stick(cx, id);
        }
    }

    fn sticky_offset(&mut self, cx: &Cx, id: NodeId) -> Option<(f32, f32)> {
        let (dx, dy) = self.scope(cx, cx.parent(id)?).offset();
        let own = self.result(id)?;
        let (port, cb) = self.sticky_frame(cx, id)?;
        let style = self.style_of(id)?;
        let origin = own.solved_origin();
        let size = own.rect();
        let node = Rect::new(
            Point::new(origin.x + dx, origin.y + dy),
            size.width,
            size.height,
        );
        let margin = NodeEdges::build(style, cb.width).margin.or_zero();
        let inset = insets(style, port);
        let x = axis_shift(
            span(node, true),
            [margin.left, margin.right],
            span(port, true),
            span(cb, true),
            [inset.left, inset.right],
        );
        let y = axis_shift(
            span(node, false),
            [margin.top, margin.bottom],
            span(port, false),
            span(cb, false),
            [inset.top, inset.bottom],
        );
        Some((snap::offset(x), snap::offset(y)))
    }

    fn sticky_frame(&self, cx: &Cx, id: NodeId) -> Option<(Rect, Rect)> {
        let mut content = None;
        let mut at = cx.parent(id);
        while let Some(ancestor) = at {
            let parent = cx.parent(ancestor);
            if let Some(result) = self.result(ancestor) {
                let rect = result.rect();
                let edges = self
                    .style_of(ancestor)
                    .map_or(NodeEdges::BARE, |style| NodeEdges::build(style, rect.width));
                let cb = *content.get_or_insert_with(|| inner(rect, &edges, true));
                if result.clips() || parent.is_none() {
                    return Some((inner(rect, &edges, false), cb));
                }
            }
            at = parent;
        }
        None
    }
}

pub(super) fn note_moved(cx: &mut Cx, id: NodeId) {
    let mut ids = SmallVec::<[NodeId; 32]>::new();
    cx.visit_preorder(id, |node, _| ids.push(node));
    for node in ids {
        cx.delta.record(node, DeltaKind::Layout);
    }
}

fn insets(style: &StyleData, port: Rect) -> Edges<Option<f32>> {
    let inset = style.inset;
    Edges {
        left: resolve_lp_auto(inset.left.into(), port.width),
        right: resolve_lp_auto(inset.right.into(), port.width),
        top: resolve_lp_auto(inset.top.into(), port.height),
        bottom: resolve_lp_auto(inset.bottom.into(), port.height),
    }
}

fn inner(rect: Rect, edges: &NodeEdges, content: bool) -> Rect {
    let pad = |side: f32| if content { side } else { 0.0 };
    let left = edges.border.left + pad(edges.padding.left);
    let right = edges.border.right + pad(edges.padding.right);
    let top = edges.border.top + pad(edges.padding.top);
    let bottom = edges.border.bottom + pad(edges.padding.bottom);
    Rect::new(
        Point::new(rect.origin.x + left, rect.origin.y + top),
        (rect.width - left - right).max(0.0),
        (rect.height - top - bottom).max(0.0),
    )
}

const fn span(rect: Rect, horizontal: bool) -> [f32; 2] {
    if horizontal {
        [rect.origin.x, rect.origin.x + rect.width]
    } else {
        [rect.origin.y, rect.origin.y + rect.height]
    }
}

fn axis_shift(
    node: [f32; 2],
    margin: [f32; 2],
    port: [f32; 2],
    cb: [f32; 2],
    inset: [Option<f32>; 2],
) -> f32 {
    let ([start, end], [margin_start, margin_end]) = (node, margin);
    let ([port_start, port_end], [cb_start, cb_end]) = (port, cb);
    if let [Some(lead), _] = inset {
        let room = (cb_end - end - margin_end).max(0.0);
        let push = (port_start + lead - start).clamp(0.0, room);
        if push > 0.0 {
            return push;
        }
    }
    let [_, trail] = inset;
    trail.map_or(0.0, |offset| {
        let room = (start - margin_start - cb_start).max(0.0);
        let pull = (end - port_end + offset).clamp(0.0, room);
        -pull
    })
}
