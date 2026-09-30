use crate::kernel::{Cx, Node, NodeId, TopLayer};
use crate::layout::LayoutEngine;
use crate::reactive::Signal;

#[derive(Clone, Copy)]
pub(crate) struct ScrollReveal {
    pub(crate) scroll_y: Signal<f32>,
    pub(crate) max: Signal<f32>,
}

pub(crate) fn reveal_focus(cx: &Cx, layout: &LayoutEngine, target: NodeId) {
    let Some(rect) = layout.result(target).map(|result| result.rect()) else {
        return;
    };
    let mut top = rect.origin.y;
    let mut bottom = top + rect.height;
    let mut current = layer_parent(cx, target);
    while let Some(id) = current {
        if let Some(reveal) = cx
            .get(id)
            .and_then(|node| node.get_extension::<ScrollReveal>())
            .copied()
            && let Some(view) = layout.result(id).map(|result| result.rect())
        {
            let (Some(before), Some(max)) = (reveal.scroll_y.try_peek(), reveal.max.try_peek())
            else {
                return;
            };
            let shift = overflow(top, bottom, view.origin.y, view.origin.y + view.height);
            let after = (before + shift).clamp(0.0, max.max(0.0));
            if (after - before).abs() > f32::EPSILON && !reveal.scroll_y.try_set(after) {
                return;
            }
            top -= after - before;
            bottom -= after - before;
        }
        current = layer_parent(cx, id);
    }
}

fn layer_parent(cx: &Cx, id: NodeId) -> Option<NodeId> {
    cx.get(id)
        .filter(|node| node.get_extension::<TopLayer>().is_none())
        .and_then(Node::parent)
}

fn overflow(top: f32, bottom: f32, view_top: f32, view_bottom: f32) -> f32 {
    if top < view_top {
        top - view_top
    } else if bottom > view_bottom {
        (bottom - view_bottom).min(top - view_top)
    } else {
        0.0
    }
}
