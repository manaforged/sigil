use core::fmt;

use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::widgets::Orientation;

/// The rule that a `Flaw` breaks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FlawKind {
    /// Rows of a table whose columns do not line up.
    Column,
    /// A lone child with nearly, but not exactly, even space around it.
    Inset,
    /// Text taller than the box that clips it.
    ClippedText,
    /// Controls in one row with different heights.
    UnevenControls,
    /// Siblings in normal flow that cover each other.
    Overlap,
    /// A child that leaves a parent that does not clip or scroll.
    Overflow,
    /// Text below the WCAG 2.2 AA contrast ratio.
    Contrast,
    /// A control under the WCAG 2.2 AA pointer target size.
    SmallTarget,
}

/// A design rule that the laid-out tree breaks.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Flaw {
    /// Rows of a table whose columns do not line up.
    Column {
        /// The stack that holds the rows.
        parent: NodeLabel,
        /// The column index.
        column: usize,
        /// The top of each row.
        tops: Vec<f32>,
        /// The start of the cell in each row.
        starts: Vec<f32>,
        /// The end of the cell in each row.
        ends: Vec<f32>,
    },
    /// A lone child with nearly, but not exactly, even space around it.
    Inset {
        /// The child.
        node: NodeLabel,
        /// The child's box.
        rect: Rect,
        /// The axis of the uneven space.
        axis: Orientation,
        /// The space before the child.
        start: f32,
        /// The space after the child.
        end: f32,
    },
    /// Text taller than the box that clips it.
    ClippedText {
        /// The text.
        node: NodeLabel,
        /// The height that the text needs.
        needed: f32,
        /// The height of the clip.
        shown: f32,
    },
    /// Controls in one row with different heights.
    UnevenControls {
        /// The row.
        parent: NodeLabel,
        /// The height of each control.
        heights: Vec<f32>,
    },
    /// Siblings in normal flow that cover each other.
    Overlap {
        /// The earlier sibling.
        first: NodeLabel,
        /// The later sibling.
        second: NodeLabel,
    },
    /// A child that leaves a parent that does not clip or scroll.
    Overflow {
        /// The child.
        node: NodeLabel,
        /// The parent.
        parent: NodeLabel,
        /// The distance outside the parent.
        by: f32,
    },
    /// Text below the WCAG 2.2 AA contrast ratio.
    Contrast {
        /// The text.
        node: NodeLabel,
        /// The ratio found.
        ratio: f32,
        /// The ratio needed.
        needed: f32,
    },
    /// A control under the WCAG 2.2 AA pointer target size.
    SmallTarget {
        /// The control.
        node: NodeLabel,
        /// The width found.
        width: f32,
        /// The height found.
        height: f32,
        /// The size needed on each side.
        needed: f32,
    },
}

impl Flaw {
    /// Returns the rule that this flaw breaks.
    #[must_use]
    pub const fn kind(&self) -> FlawKind {
        match self {
            Self::Column { .. } => FlawKind::Column,
            Self::Inset { .. } => FlawKind::Inset,
            Self::ClippedText { .. } => FlawKind::ClippedText,
            Self::UnevenControls { .. } => FlawKind::UnevenControls,
            Self::Overlap { .. } => FlawKind::Overlap,
            Self::Overflow { .. } => FlawKind::Overflow,
            Self::Contrast { .. } => FlawKind::Contrast,
            Self::SmallTarget { .. } => FlawKind::SmallTarget,
        }
    }

    /// Returns the node to fix.
    #[must_use]
    pub const fn node(&self) -> &NodeLabel {
        match self {
            Self::Column { parent, .. } | Self::UnevenControls { parent, .. } => parent,
            Self::Overlap { first, .. } => first,
            Self::Inset { node, .. }
            | Self::ClippedText { node, .. }
            | Self::Overflow { node, .. }
            | Self::Contrast { node, .. }
            | Self::SmallTarget { node, .. } => node,
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

impl Flaw {
    fn fmt_column(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Column {
            parent,
            column,
            tops,
            starts,
            ends,
        } = self
        else {
            return Ok(());
        };
        write!(
            f,
            "column {column} of {parent} in rows at y {tops:?} starts at {starts:?} and ends at {ends:?}: give the cells of the column one width, or use a grid"
        )
    }

    fn fmt_inset(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Inset {
            node,
            rect,
            axis,
            start,
            end,
        } = self
        else {
            return Ok(());
        };
        write!(
            f,
            "{node} at ({}, {}) sized {}x{} is inset {start} at the start and {end} at the end on axis {axis:?}",
            rect.origin.x, rect.origin.y, rect.width, rect.height
        )
    }
}

impl fmt::Display for Flaw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Column { .. } => self.fmt_column(f),
            Self::Inset { .. } => self.fmt_inset(f),
            Self::ClippedText {
                node,
                needed,
                shown,
            } => write!(
                f,
                "{node} needs {needed} of height and its clip shows {shown}: let the box grow, or use a smaller TextRole"
            ),
            Self::UnevenControls { parent, heights } => write!(
                f,
                "the controls in {parent} have heights {heights:?}: give them one ControlSize"
            ),
            Self::Overlap { first, second } => {
                write!(
                    f,
                    "{first} and {second} cover each other: check their sizes and margins"
                )
            }
            Self::Overflow { node, parent, by } => write!(
                f,
                "{node} leaves {parent} by {by}: let the parent grow, wrap the content, or scroll it"
            ),
            Self::Contrast {
                node,
                ratio,
                needed,
            } => write!(
                f,
                "{node} has a contrast of {ratio:.2}:1 and needs {needed}:1: use a theme Token"
            ),
            Self::SmallTarget {
                node,
                width,
                height,
                needed,
            } => write!(
                f,
                "{node} is {width}x{height} and needs {needed} on each side: use a larger ControlSize"
            ),
        }
    }
}
