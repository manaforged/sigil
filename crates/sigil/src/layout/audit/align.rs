use crate::geometry::Rect;
use crate::kernel::{Cx, Node, NodeId, NodeType};
use crate::layout::{Display, FlexDirection, FlexWrap, LayoutEngine, LayoutStyle, StyleData};
use crate::widgets::Orientation;

use super::{Issue, Tolerance, is_row, label, style};

fn is_stack(node: &Node) -> bool {
    matches!(
        style(node),
        (Display::Block, _, _)
            | (
                Display::Flex,
                FlexDirection::Column | FlexDirection::ColumnReverse,
                FlexWrap::NoWrap
            )
    )
}

#[derive(PartialEq)]
struct RowShape {
    style: Option<StyleData>,
    cells: Vec<core::mem::Discriminant<NodeType>>,
}

fn shape(cx: &Cx, row: &Node) -> RowShape {
    RowShape {
        style: row
            .get_extension::<LayoutStyle>()
            .map(|style| style.data().clone()),
        cells: row
            .children()
            .iter()
            .filter_map(|child| cx.get(*child))
            .map(|cell| core::mem::discriminant(&cell.kind))
            .collect(),
    }
}

fn runs<'a>(cx: &'a Cx, node: &Node) -> Vec<Vec<&'a Node>> {
    let mut runs: Vec<(Option<RowShape>, Vec<&Node>)> = Vec::new();
    for child in node.children().iter().filter_map(|child| cx.get(*child)) {
        let fits = is_row(child) && !child.children().is_empty();
        let form = fits.then(|| shape(cx, child));
        match runs.last_mut() {
            Some((Some(last), run)) if form.as_ref() == Some(last) => run.push(child),
            _ => runs.push((form, vec![child])),
        }
    }
    runs.into_iter()
        .filter(|(_, run)| run.len() >= 2)
        .map(|(_, run)| run)
        .collect()
}

fn spread(values: impl Iterator<Item = f32>) -> f32 {
    let (low, high) = values.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    high - low
}

pub(super) fn columns(
    cx: &Cx,
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    tolerance: Tolerance,
    out: &mut Vec<Issue>,
) {
    if !is_stack(node) {
        return;
    }
    for rows in runs(cx, node) {
        run_columns(layout, (id, node), &rows, tolerance, out);
    }
}

fn run_columns(
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    rows: &[&Node],
    tolerance: Tolerance,
    out: &mut Vec<Issue>,
) {
    let width = rows.first().map_or(0, |row| row.children().len());
    for column in 0..width {
        let rects: Option<Vec<Rect>> = rows
            .iter()
            .map(|row| {
                row.children()
                    .get(column)
                    .and_then(|child| layout.result(*child))
                    .map(|r| r.rect())
            })
            .collect();
        let Some(rects) = rects else {
            continue;
        };
        let starts: Vec<f32> = rects.iter().map(|r| r.origin.x).collect();
        let ends: Vec<f32> = rects.iter().map(|r| r.origin.x + r.width).collect();
        let centers = rects.iter().map(|r| r.origin.x + r.width / 2.0);
        let aligned = [
            spread(starts.iter().copied()),
            spread(ends.iter().copied()),
            spread(centers),
        ]
        .iter()
        .any(|gap| *gap <= tolerance.exact);
        if !aligned {
            out.push(Issue::Column {
                parent: label(id, node),
                column,
                tops: rects.iter().map(|r| r.origin.y).collect(),
                starts,
                ends,
            });
        }
    }
}

pub(super) fn insets(
    cx: &Cx,
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    tolerance: Tolerance,
    out: &mut Vec<Issue>,
) {
    if matches!(node.kind, NodeType::Text { .. }) {
        return;
    }
    let Some(parent) = node.parent().and_then(|parent| cx.get(parent)) else {
        return;
    };
    if parent.children().len() != 1 {
        return;
    }
    let outer = node.parent().and_then(|above| layout.result(above));
    let (Some(outer), Some(inner)) = (outer, layout.result(id)) else {
        return;
    };
    let (outer, inner) = (outer.rect(), inner.rect());
    let spans = [
        (
            Orientation::Horizontal,
            inner.origin.x - outer.origin.x,
            (outer.origin.x + outer.width) - (inner.origin.x + inner.width),
        ),
        (
            Orientation::Vertical,
            inner.origin.y - outer.origin.y,
            (outer.origin.y + outer.height) - (inner.origin.y + inner.height),
        ),
    ];
    for (axis, start, end) in spans {
        let gap = (start - end).abs();
        if start > 0.0 && end > 0.0 && gap > tolerance.exact && gap <= tolerance.near_miss {
            out.push(Issue::Inset {
                node: label(id, node),
                rect: inner,
                axis,
                start,
                end,
            });
        }
    }
}
