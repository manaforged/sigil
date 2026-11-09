use crate::geometry::Rect;
use crate::kernel::{Cx, HostMode, Node, NodeId, NodeType};
use crate::layout::{Display, FlexDirection, FlexWrap, LayoutEngine, LayoutStyle, StyleData};
use crate::style::Theme;
use crate::widgets::Orientation;

pub use crate::layout::builder_ext::Asymmetric;

mod contrast;
mod flaw;
mod rules;

pub use flaw::{Flaw, FlawKind, NodeLabel};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    pub exact: f32,
    pub near_miss: f32,
    pub target: f32,
    pub boxed_controls: bool,
}

impl Tolerance {
    #[must_use]
    pub const fn cells() -> Self {
        Self {
            exact: crate::Host::CELL,
            near_miss: 3.0 * crate::Host::CELL,
            target: 0.0,
            boxed_controls: false,
        }
    }

    #[must_use]
    pub const fn points() -> Self {
        Self {
            exact: 0.5,
            near_miss: 4.0,
            target: 24.0,
            boxed_controls: true,
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

/// Lists the design rules that the laid-out tree breaks.
#[must_use]
pub fn flaws(
    cx: &Cx,
    layout: &LayoutEngine,
    (root, theme): (NodeId, &Theme),
    tolerance: Tolerance,
) -> Vec<Flaw> {
    let mut out = Vec::new();
    cx.visit_preorder(root, |id, node| {
        if node.get_extension::<Asymmetric>().is_none() {
            columns(cx, layout, (id, node), tolerance, &mut out);
            insets(cx, layout, (id, node), tolerance, &mut out);
        }
        rules::clipped_text(cx, layout, (id, node), &mut out);
        rules::uneven_controls(cx, layout, (id, node), tolerance, &mut out);
        rules::overlap(cx, layout, node, &mut out);
        rules::escaped(cx, layout, (id, node), &mut out);
        rules::small_target(layout, (id, node), tolerance, &mut out);
    });
    contrast::contrast(cx, layout, root, theme, &mut out);
    out.retain(|flaw| !allowed(cx, flaw));
    out
}

fn allowed(cx: &Cx, flaw: &Flaw) -> bool {
    let mut at = Some(flaw.node().id);
    while let Some(id) = at {
        let Some(node) = cx.get(id) else {
            return false;
        };
        if node
            .get_extension::<AllowedFlaws>()
            .is_some_and(|kinds| kinds.0.contains(&flaw.kind()))
        {
            return true;
        }
        at = node.parent();
    }
    false
}

/// The rules that a node and its children opt out of.
#[derive(Clone, Debug, Default)]
pub(crate) struct AllowedFlaws(pub(crate) smallvec::SmallVec<[FlawKind; 2]>);

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

fn columns(
    cx: &Cx,
    layout: &LayoutEngine,
    (id, node): (NodeId, &Node),
    tolerance: Tolerance,
    out: &mut Vec<Flaw>,
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
    out: &mut Vec<Flaw>,
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
            out.push(Flaw::Column {
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
    out: &mut Vec<Flaw>,
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
            out.push(Flaw::Inset {
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
#[path = "audit_tests.rs"]
mod tests;
