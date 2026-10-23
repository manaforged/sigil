use core::fmt;

use crate::geometry::Rect;
use crate::kernel::{Cx, HostMode, Node, NodeId, NodeType};
use crate::layout::{Display, FlexDirection, FlexWrap, LayoutEngine, LayoutStyle, StyleData};
use crate::widgets::Axis;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    pub exact: f32,
    pub near_miss: f32,
}

impl Tolerance {
    #[must_use]
    pub const fn cells() -> Self {
        Self {
            exact: 0.0,
            near_miss: 3.0 * crate::Host::CELL,
        }
    }

    #[must_use]
    pub const fn points() -> Self {
        Self {
            exact: 0.5,
            near_miss: 4.0,
        }
    }

    #[must_use]
    pub const fn for_mode(mode: HostMode) -> Self {
        match mode {
            HostMode::Cell => Self::cells(),
            HostMode::Pixel => Self::points(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Asymmetric(pub &'static str);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MisalignmentKind {
    Column,
    Inset,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Misalignment {
    Column {
        parent: NodeLabel,
        column: usize,
        tops: Vec<f32>,
        starts: Vec<f32>,
        ends: Vec<f32>,
    },
    Inset {
        node: NodeLabel,
        rect: Rect,
        axis: Axis,
        start: f32,
        end: f32,
    },
}

impl Misalignment {
    #[must_use]
    pub const fn kind(&self) -> MisalignmentKind {
        match self {
            Self::Column { .. } => MisalignmentKind::Column,
            Self::Inset { .. } => MisalignmentKind::Inset,
        }
    }

    #[must_use]
    pub const fn node(&self) -> &NodeLabel {
        match self {
            Self::Column { parent, .. } => parent,
            Self::Inset { node, .. } => node,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeLabel {
    pub id: NodeId,
    pub label: String,
}

impl fmt::Display for NodeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} #{}>", self.label, self.id.to_accesskit().0)
    }
}

impl fmt::Display for Misalignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Column {
                parent,
                column,
                tops,
                starts,
                ends,
            } => write!(
                f,
                "column {column} of {parent} in rows at y {tops:?} starts at {starts:?} and ends at {ends:?}"
            ),
            Self::Inset {
                node,
                rect,
                axis,
                start,
                end,
            } => write!(
                f,
                "{node} at ({}, {}) sized {}x{} is inset {start} at the start and {end} at the end on axis {axis:?}",
                rect.origin.x, rect.origin.y, rect.width, rect.height
            ),
        }
    }
}

#[must_use]
pub fn misalignments(
    cx: &Cx,
    layout: &LayoutEngine,
    root: NodeId,
    tolerance: Tolerance,
) -> Vec<Misalignment> {
    let mut out = Vec::new();
    cx.visit_preorder(root, |id, node| {
        if node.get_extension::<Asymmetric>().is_some() {
            return;
        }
        columns(cx, layout, (id, node), tolerance, &mut out);
        insets(cx, layout, (id, node), tolerance, &mut out);
    });
    out
}

fn label(id: NodeId, node: &Node) -> NodeLabel {
    let kind = match &node.kind {
        NodeType::Container => "Container",
        NodeType::Fragment => "Fragment",
        NodeType::Text { .. } => "Text",
        NodeType::Image { .. } => "Image",
    };
    NodeLabel {
        id,
        label: format!("{kind} role={:?}", node.a11y.role),
    }
}

fn style(node: &Node) -> (Display, FlexDirection, FlexWrap) {
    node.get_extension::<LayoutStyle>().map_or(
        (Display::Block, FlexDirection::Column, FlexWrap::NoWrap),
        |style| {
            let data = style.data();
            (data.display, data.flex_direction, data.flex_wrap)
        },
    )
}

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

fn is_row(node: &Node) -> bool {
    matches!(
        style(node),
        (
            Display::Flex,
            FlexDirection::Row | FlexDirection::RowReverse,
            FlexWrap::NoWrap
        )
    )
}

#[derive(PartialEq)]
struct Shape {
    style: Option<StyleData>,
    cells: Vec<core::mem::Discriminant<NodeType>>,
}

fn shape(cx: &Cx, row: &Node) -> Shape {
    Shape {
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
    let mut runs: Vec<(Option<Shape>, Vec<&Node>)> = Vec::new();
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

fn columns(
    cx: &Cx,
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    tolerance: Tolerance,
    out: &mut Vec<Misalignment>,
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
    out: &mut Vec<Misalignment>,
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
            out.push(Misalignment::Column {
                parent: label(id, node),
                column,
                tops: rects.iter().map(|r| r.origin.y).collect(),
                starts,
                ends,
            });
        }
    }
}

fn insets(
    cx: &Cx,
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    tolerance: Tolerance,
    out: &mut Vec<Misalignment>,
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
            Axis::X,
            inner.origin.x - outer.origin.x,
            (outer.origin.x + outer.width) - (inner.origin.x + inner.width),
        ),
        (
            Axis::Y,
            inner.origin.y - outer.origin.y,
            (outer.origin.y + outer.height) - (inner.origin.y + inner.height),
        ),
    ];
    for (axis, start, end) in spans {
        let gap = (start - end).abs();
        if start > 0.0 && end > 0.0 && gap > tolerance.exact && gap <= tolerance.near_miss {
            out.push(Misalignment::Inset {
                node: label(id, node),
                rect: inner,
                axis,
                start,
                end,
            });
        }
    }
}

#[cfg(test)]
#[path = "align_tests.rs"]
mod tests;
