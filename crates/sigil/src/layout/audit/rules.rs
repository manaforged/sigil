use crate::geometry::Rect;
use crate::kernel::{Cx, Node, NodeId, NodeType, Role};
use crate::layout::{LayoutEngine, LayoutStyle, Overflow, Position, ReactivePosition};

use super::{Flaw, Tolerance, is_row, label};

const SLACK: f32 = 1.0;

fn rect(layout: &LayoutEngine, id: NodeId) -> Option<Rect> {
    layout
        .result(id)
        .map(|result| result.rect())
        .filter(|rect| rect.width > 0.0 && rect.height > 0.0)
}

fn in_flow(node: &Node) -> bool {
    node.get_extension::<ReactivePosition>().is_none()
        && node
            .get_extension::<LayoutStyle>()
            .is_none_or(|style| style.data().position != Position::Absolute)
}

fn overflow(node: &Node) -> (Overflow, Overflow) {
    node.get_extension::<LayoutStyle>()
        .map_or((Overflow::Visible, Overflow::Visible), |style| {
            let axes = style.data().overflow;
            (axes.width, axes.height)
        })
}

const fn is_control(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::CheckBox
            | Role::RadioButton
            | Role::Switch
            | Role::TextInput
            | Role::PasswordInput
            | Role::SearchInput
            | Role::EmailInput
            | Role::NumberInput
    )
}

const fn is_boxed_control(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::TextInput
            | Role::PasswordInput
            | Role::SearchInput
            | Role::EmailInput
            | Role::NumberInput
    )
}

pub(super) fn clipped_text(
    cx: &Cx,
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    out: &mut Vec<Flaw>,
) {
    if !matches!(node.kind, NodeType::Text { .. }) {
        return;
    }
    let Some(shown) = rect(layout, id) else {
        return;
    };
    let mut above = node.parent();
    while let Some(parent) = above {
        let Some(holder) = cx.get(parent) else {
            return;
        };
        if overflow(holder).1 == Overflow::Hidden {
            if let Some(clip) = rect(layout, parent)
                && clip.height + 0.5 < shown.height
            {
                out.push(Flaw::ClippedText {
                    node: label(id, node),
                    needed: shown.height,
                    shown: clip.height,
                });
            }
            return;
        }
        above = holder.parent();
    }
}

pub(super) fn uneven_controls(
    cx: &Cx,
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    out: &mut Vec<Flaw>,
) {
    if !is_row(node) {
        return;
    }
    let heights: Vec<f32> = node
        .children()
        .iter()
        .filter_map(|child| Some((*child, cx.get(*child)?)))
        .filter(|(_, child)| is_boxed_control(child.a11y.role))
        .filter_map(|(child, _)| rect(layout, child))
        .map(|rect| rect.height)
        .collect();
    let (low, high) = heights
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), height| {
            (low.min(*height), high.max(*height))
        });
    if heights.len() >= 2 && high - low > 0.5 {
        out.push(Flaw::UnevenControls {
            parent: label(id, node),
            heights,
        });
    }
}

fn intersects(a: Rect, b: Rect) -> bool {
    let wide =
        a.origin.x.max(b.origin.x) + SLACK < (a.origin.x + a.width).min(b.origin.x + b.width);
    let tall =
        a.origin.y.max(b.origin.y) + SLACK < (a.origin.y + a.height).min(b.origin.y + b.height);
    wide && tall
}

pub(super) fn overlap(cx: &Cx, layout: &LayoutEngine, node: &Node, out: &mut Vec<Flaw>) {
    let flow: Vec<(NodeId, &Node, Rect)> = node
        .children()
        .iter()
        .filter_map(|child| Some((*child, cx.get(*child)?, rect(layout, *child)?)))
        .filter(|(_, child, _)| in_flow(child))
        .collect();
    for pair in flow.windows(2) {
        if let [(first, first_node, a), (second, second_node, b)] = pair
            && intersects(*a, *b)
        {
            out.push(Flaw::Overlap {
                first: label(*first, first_node),
                second: label(*second, second_node),
            });
        }
    }
}

pub(super) fn escaped(
    cx: &Cx,
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    out: &mut Vec<Flaw>,
) {
    if !in_flow(node) {
        return;
    }
    let Some(parent_id) = node.parent() else {
        return;
    };
    let Some(parent) = cx.get(parent_id) else {
        return;
    };
    if overflow(parent) != (Overflow::Visible, Overflow::Visible) {
        return;
    }
    let (Some(inner), Some(outer)) = (rect(layout, id), rect(layout, parent_id)) else {
        return;
    };
    let by = [
        outer.origin.x - inner.origin.x,
        outer.origin.y - inner.origin.y,
        inner.origin.x + inner.width - outer.origin.x - outer.width,
        inner.origin.y + inner.height - outer.origin.y - outer.height,
    ]
    .into_iter()
    .fold(0.0_f32, f32::max);
    if by > SLACK {
        out.push(Flaw::Overflow {
            node: label(id, node),
            parent: label(parent_id, parent),
            by,
        });
    }
}

pub(super) fn small_target(
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    tolerance: Tolerance,
    out: &mut Vec<Flaw>,
) {
    if !is_control(node.a11y.role) {
        return;
    }
    let Some(target) = rect(layout, id) else {
        return;
    };
    if target.width < tolerance.target || target.height < tolerance.target {
        out.push(Flaw::SmallTarget {
            node: label(id, node),
            width: target.width,
            height: target.height,
            needed: tolerance.target,
        });
    }
}
