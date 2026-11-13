use crate::kernel::{Cx, HostMode, Node, NodeId, NodeType};
use crate::layout::{Display, FlexDirection, FlexWrap, LayoutEngine, LayoutStyle};
use crate::style::Theme;

mod align;
mod contrast;
mod flaw;
mod rules;

pub use flaw::{Flaw, Issue, NodeLabel};

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
) -> Vec<Issue> {
    let mut out = Vec::new();
    cx.visit_preorder(root, |id, node| {
        align::columns(cx, layout, (id, node), tolerance, &mut out);
        align::insets(cx, layout, (id, node), tolerance, &mut out);
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

fn allowed(cx: &Cx, flaw: &Issue) -> bool {
    let mut at = Some(flaw.node().id);
    while let Some(id) = at {
        let Some(node) = cx.get(id) else {
            return false;
        };
        if node
            .extension::<AllowedFlaws>()
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
pub(crate) struct AllowedFlaws(pub(crate) smallvec::SmallVec<[Flaw; 2]>);

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
    node.extension::<LayoutStyle>().map_or(
        (Display::Block, FlexDirection::Column, FlexWrap::NoWrap),
        |style| {
            let data = style.data();
            (data.display, data.flex_direction, data.flex_wrap)
        },
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

#[cfg(test)]
#[path = "audit_tests.rs"]
mod tests;
